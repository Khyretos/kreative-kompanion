//! One ComfyUI instance: queue an API-format graph, wait for it, fetch its output files.
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// How long a ComfyUI may take to come up (GPU-01: a stopped app is started by the runner on
/// its next report, up to 60 s, and then boots).
const READY_WAIT: Duration = Duration::from_secs(300);

/// BUG-02: wait for a queued prompt; fails at `timeout`, or when ComfyUI's queue has not listed
/// it for `stall` (it crashed, restarted or lost the prompt).
pub async fn wait(http: &reqwest::Client, base: &str, id: &str, timeout: Duration, stall: Duration) -> Result<Value> {
    let started = Instant::now();
    let mut progress = super::watchdog::Progress::new(stall, Instant::now());
    let mut checked = Instant::now();
    let h = loop {
        if started.elapsed() > timeout {
            bail!("ComfyUI did not finish in {} s", timeout.as_secs());
        }
        let v: Value = http
            .get(format!("{base}/history/{id}"))
            .timeout(Duration::from_secs(20))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let h = &v[id];
        if h["status"]["status_str"] == "error" {
            let msg = h["status"]["messages"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|m| m[0] == "execution_error")
                .and_then(|m| m[1]["exception_message"].as_str())
                .unwrap_or("execution failed")
                .to_string();
            bail!("ComfyUI: {msg}");
        }
        if h["status"]["completed"] == true {
            break h.clone();
        }
        if checked.elapsed() >= stall / 9 {
            checked = Instant::now();
            let alive = match http.get(format!("{base}/queue")).timeout(Duration::from_secs(10)).send().await {
                Ok(r) => r.json::<Value>().await.is_ok_and(|q| super::watchdog::in_queue(&q, id)),
                Err(_) => false,
            };
            progress.tick(alive, Instant::now())?;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    Ok(h)
}

pub async fn run(
    http: &reqwest::Client,
    base: &str,
    graph: &Value,
    outputs: &[String],
    out_dir: &Path,
    stem: &str,
    timeout: Duration,
) -> Result<Vec<PathBuf>> {
    // A ComfyUI the role switch just started needs a few seconds before it takes requests.
    let started = Instant::now();
    while !http
        .get(format!("{base}/system_stats"))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success())
    {
        if started.elapsed() > READY_WAIT {
            bail!(
                "ComfyUI at {base} did not answer in {} s",
                READY_WAIT.as_secs()
            );
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    let resp = http
        .post(format!("{base}/prompt"))
        .json(&json!({"prompt": graph, "client_id": stem}))
        .send()
        .await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!(
            "ComfyUI refused the graph: {}",
            body.chars().take(500).collect::<String>()
        );
    }
    let v: Value = resp.json().await?;
    let id = v["prompt_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("ComfyUI sent no prompt_id"))?
        .to_string();

    let h = wait(http, base, &id, timeout, super::watchdog::STALL).await?;

    tokio::fs::create_dir_all(out_dir).await?;
    shared(out_dir, 0o2775).await;
    let mut files = Vec::new();
    for node in outputs {
        for key in &["images", "audio", "gifs", "videos"] {
            if let Some(arr) = h["outputs"][node].get(key).and_then(|v| v.as_array()) {
                for item in arr {
                    let filename = item["filename"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing filename"))?;
                    let subfolder = item["subfolder"].as_str().unwrap_or("");
                    let kind = item["type"].as_str().unwrap_or("output");
                    let resp = http
                        .get(format!("{base}/view"))
                        .query(&[
                            ("filename", filename),
                            ("subfolder", subfolder),
                            ("type", kind),
                        ])
                        .send()
                        .await?
                        .error_for_status()?;
                    let bytes = resp.bytes().await?;
                    let ext = Path::new(filename)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "bin".to_string());
                    let path = out_dir.join(format!("{stem}-{}.{ext}", files.len() + 1));
                    tokio::fs::write(&path, &bytes).await?;
                    shared(&path, 0o664).await;
                    files.push(path);
                }
            }
        }
    }
    if files.is_empty() {
        bail!("ComfyUI finished without outputs");
    }
    Ok(files)
}

/// Outputs go to a folder people share (/media/Generated): its group may change them, and a
/// setgid folder hands its group on to new files.
pub async fn shared(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).await;
}

/// STU-C3: host RAM (MiB) that a ComfyUI already holds, from its GET /system_stats answer.
/// Only a ComfyUI in a container with a memory limit reports its own RAM there; one that sees
/// the whole host (its total within 1 GiB of the host's) gives 0, and so do missing numbers.
pub fn ram_held_mib(stats: &Value, host_total_mib: u64) -> u64 {
    let Some(ram_total) = stats["system"]["ram_total"].as_u64() else { return 0 };
    let Some(ram_free) = stats["system"]["ram_free"].as_u64() else { return 0 };
    if host_total_mib == 0 || ram_total / 1_048_576 + 1024 >= host_total_mib {
        return 0;
    }
    ram_total.saturating_sub(ram_free) / 1_048_576
}

/// Ask an idle ComfyUI to drop its cached models, so the VRAM ledger sees room for the next
/// job on it. True when it was idle and took the request.
pub async fn free_if_idle(http: &reqwest::Client, base: &str) -> bool {
    let Ok(r) = http.get(format!("{base}/queue")).send().await else {
        return false;
    };
    let Ok(v) = r.json::<Value>().await else {
        return false;
    };
    let empty = |k: &str| v[k].as_array().is_none_or(|a| a.is_empty());
    if !empty("queue_running") || !empty("queue_pending") {
        return false;
    }
    let free = http
        .post(format!("{base}/free"))
        .json(&json!({"unload_models": true, "free_memory": true}))
        .send()
        .await;
    free.is_ok_and(|r| r.status().is_success())
}


pub async fn upload(http: &reqwest::Client, base: &str, bytes: Vec<u8>, name: &str) -> Result<String> {
    let started = Instant::now();
    while !http
        .get(format!("{base}/system_stats"))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success())
    {
        if started.elapsed() > READY_WAIT {
            bail!(
                "ComfyUI at {base} did not answer in {} s",
                READY_WAIT.as_secs()
            );
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let form = reqwest::multipart::Form::new()
        .part("image", reqwest::multipart::Part::bytes(bytes).file_name(name.to_string()))
        .text("subfolder", "kompanion")
        .text("type", "input")
        .text("overwrite", "true");

    let resp = http.post(format!("{base}/upload/image")).multipart(form).send().await?;
    if !resp.status().is_success() {
        bail!("ComfyUI refused the photo: {}", resp.status());
    }

    let v: Value = resp.json().await?;
    let file = v["name"].as_str().ok_or_else(|| anyhow::anyhow!("ComfyUI sent no file name"))?;
    let sub = v["subfolder"].as_str().unwrap_or("");

    Ok(if sub.is_empty() {
        file.to_string()
    } else {
        format!("{sub}/{file}")
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::{Query, State},
        http::StatusCode,
        response::{IntoResponse, Response},
        routing::{get, post},
    };
    use reqwest::Client;
    use std::{
        collections::HashMap,
        future::IntoFuture,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    struct Fake {
        mode: &'static str,
        polls: AtomicUsize,
        stats: AtomicUsize,
        got: Mutex<Option<Value>>,
        freed: Mutex<Option<Value>>,
    }

    async fn prompt(State(f): State<Arc<Fake>>, Json(body): Json<Value>) -> Response {
        *f.got.lock().unwrap() = Some(body);
        if f.mode == "refuse" {
            return (StatusCode::BAD_REQUEST, "bad node").into_response();
        }
        Json(json!({"prompt_id": "p1"})).into_response()
    }

    /// Not ready on the first call, like a ComfyUI that is still starting.
    async fn stats(State(f): State<Arc<Fake>>) -> StatusCode {
        if f.stats.fetch_add(1, Ordering::SeqCst) == 0 {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::OK
        }
    }

    async fn history(State(f): State<Arc<Fake>>) -> Json<Value> {
        if f.polls.fetch_add(1, Ordering::SeqCst) == 0 || f.mode == "lost" || f.mode == "running" {
            return Json(json!({}));
        }
        if f.mode == "error" {
            return Json(
                json!({"p1": {"status": {"status_str": "error", "completed": false,
                "messages": [["execution_start", {}], ["execution_error", {"exception_message": "OOM"}]]}}}),
            );
        }
        Json(
            json!({"p1": {"status": {"status_str": "success", "completed": true},
            "outputs": {"10": {"images": [{"filename": "a.png", "subfolder": "kompanion", "type": "output"}]}}}}),
        )
    }

    /// mode "busy": one prompt running.
    async fn queue(State(f): State<Arc<Fake>>) -> Json<Value> {
        let running = if f.mode == "busy" {
            json!([[0, "x"]])
        } else if f.mode == "running" {
            json!([[0, "p1"]])
        } else {
            json!([])
        };
        Json(json!({"queue_running": running, "queue_pending": []}))
    }

    async fn free(State(f): State<Arc<Fake>>, Json(body): Json<Value>) -> StatusCode {
        *f.freed.lock().unwrap() = Some(body);
        StatusCode::OK
    }

    async fn view(Query(q): Query<HashMap<String, String>>) -> Response {
        if q.get("filename").map(String::as_str) == Some("a.png")
            && q.get("subfolder").map(String::as_str) == Some("kompanion")
        {
            return b"PNG".to_vec().into_response();
        }
        StatusCode::NOT_FOUND.into_response()
    }

    /// A fake ComfyUI on a free port: its base URL and its state.
    async fn fake(mode: &'static str) -> (String, Arc<Fake>) {
        let state = Arc::new(Fake {
            mode,
            polls: AtomicUsize::new(0),
            stats: AtomicUsize::new(0),
            got: Mutex::new(None),
            freed: Mutex::new(None),
        });
        let app = Router::new()
            .route("/prompt", post(prompt))
            .route("/history/{id}", get(history))
            .route("/view", get(view))
            .route("/system_stats", get(stats))
            .route("/queue", get(queue))
            .route("/free", post(free))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(axum::serve(listener, app).into_future());
        (url, state)
    }

    #[tokio::test]
    async fn test_ok() {
        let dir = std::env::temp_dir().join(format!("m605c-{}", uuid::Uuid::new_v4()));
        let graph = json!({"10": {"class_type": "SaveImage", "inputs": {}}});
        let outs = vec!["10".to_string()];
        let http = Client::new();
        let (base, f) = fake("ok").await;
        let files = run(
            &http,
            &base,
            &graph,
            &outs,
            &dir,
            "run1",
            Duration::from_secs(10),
        )
        .await
        .unwrap();
        assert_eq!(files, vec![dir.join("run1-1.png")]);
        assert_eq!(std::fs::read(&files[0]).unwrap(), b"PNG");
        assert_eq!(f.got.lock().unwrap().as_ref().unwrap()["prompt"], graph);
    }

    #[tokio::test]
    async fn test_error() {
        let dir = std::env::temp_dir().join(format!("m605c-{}", uuid::Uuid::new_v4()));
        let graph = json!({"10": {"class_type": "SaveImage", "inputs": {}}});
        let outs = vec!["10".to_string()];
        let http = Client::new();
        let (base, _) = fake("error").await;
        let e = run(
            &http,
            &base,
            &graph,
            &outs,
            &dir,
            "run1",
            Duration::from_secs(10),
        )
        .await
        .unwrap_err();
        assert_eq!(e.to_string(), "ComfyUI: OOM");
    }

    #[tokio::test]
    async fn test_refused() {
        let dir = std::env::temp_dir().join(format!("m605c-{}", uuid::Uuid::new_v4()));
        let graph = json!({"10": {"class_type": "SaveImage", "inputs": {}}});
        let outs = vec!["10".to_string()];
        let http = Client::new();
        let (base, _) = fake("refuse").await;
        let e = run(
            &http,
            &base,
            &graph,
            &outs,
            &dir,
            "run1",
            Duration::from_secs(10),
        )
        .await
        .unwrap_err();
        assert_eq!(e.to_string(), "ComfyUI refused the graph: bad node");
    }

    #[tokio::test]
    async fn frees_when_idle() {
        let (base, f) = fake("ok").await;
        assert!(free_if_idle(&Client::new(), &base).await);
        assert_eq!(
            f.freed.lock().unwrap().clone(),
            Some(json!({"unload_models": true, "free_memory": true}))
        );
    }

    #[tokio::test]
    async fn keeps_when_busy() {
        let (base, f) = fake("busy").await;
        assert!(!free_if_idle(&Client::new(), &base).await);
        assert!(f.freed.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn false_when_down() {
        assert!(!free_if_idle(&Client::new(), "http://127.0.0.1:1").await);
    }

    #[tokio::test]
    async fn upload_sends_the_photo() {
        use axum::extract::Multipart;
        use serde_json::json;

        async fn up(mut form: Multipart) -> Json<serde_json::Value> {
            let mut got = serde_json::Map::new();
            while let Some(f) = form.next_field().await.unwrap() {
                let name = f.name().unwrap_or("").to_string();
                let file = f.file_name().map(str::to_string);
                let text = String::from_utf8_lossy(&f.bytes().await.unwrap()).to_string();
                got.insert(name.clone(), json!(text));
                if let Some(file) = file { got.insert(format!("{name}_file"), json!(file)); }
            }
            Json(json!({"name": got["image_file"], "subfolder": got["subfolder"], "type": got["type"], "got": got}))
        }
        let app = Router::new().route("/system_stats", get(|| async { "{}" })).route("/upload/image", post(up));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(axum::serve(listener, app).into_future());

        let name = upload(&reqwest::Client::new(), &url, b"PNGDATA".to_vec(), "r1.png").await.unwrap();
        assert_eq!(name, "kompanion/r1.png");

        let bad_app = Router::new().route("/system_stats", get(|| async { "{}" }));
        let bad_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url2 = format!("http://{}", bad_listener.local_addr().unwrap());
        tokio::spawn(axum::serve(bad_listener, bad_app).into_future());

        assert!(upload(&reqwest::Client::new(), &url2, vec![1], "r2.png").await.unwrap_err().to_string().contains("ComfyUI refused the photo"));
    }

    /// BUG-02: a prompt ComfyUI lost (not in its queue, no history) fails after the stall.
    #[tokio::test]
    async fn a_lost_prompt_fails_after_the_stall() {
        let (url, _f) = fake("lost").await;
        let t = Instant::now();
        let e = wait(&Client::new(), &url, "p1", Duration::from_secs(30), Duration::from_millis(900)).await.unwrap_err().to_string();
        assert!(e.contains("no progress"), "{e}");
        assert!(t.elapsed() < Duration::from_secs(4), "{:?}", t.elapsed());
    }

    /// BUG-02: a prompt that stays running ends at the time limit.
    #[tokio::test]
    async fn a_running_prompt_ends_at_the_limit() {
        let (url, _f) = fake("running").await;
        let e = wait(&Client::new(), &url, "p1", Duration::from_millis(1500), Duration::from_millis(900)).await.unwrap_err().to_string();
        assert!(e.contains("did not finish"), "{e}");
    }

    /// STU-C3: ComfyUI in a container with a memory limit reports the container's own RAM;
    /// one that sees the whole host reports the host's, which is not ComfyUI's to give back.
    #[test]
    fn ram_held_counts_only_a_containers_own_ram() {
        let gib = 1u64 << 30;
        let stats = |total: u64, free: u64| json!({"system": {"ram_total": total, "ram_free": free}});
        // soucouyant, 2026-10-09: 20 GiB limit, 14.57 GiB free in it, host 31692 MiB.
        assert_eq!(ram_held_mib(&stats(20 * gib, 15647371264), 31692), 5557);
        // Same total as the host (no limit): nothing.
        assert_eq!(ram_held_mib(&stats(62 * gib, 7 * gib), 64202), 0);
        // Missing numbers or an unknown host: nothing.
        assert_eq!(ram_held_mib(&json!({}), 31692), 0);
        assert_eq!(ram_held_mib(&stats(20 * gib, 15 * gib), 0), 0);
    }
}
