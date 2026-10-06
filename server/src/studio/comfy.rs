//! One ComfyUI instance: queue an API-format graph, wait for it, fetch its output files.
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// How long a ComfyUI may take to come up.
const READY_WAIT: Duration = Duration::from_secs(180);

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

    let started = Instant::now();
    let h = loop {
        if started.elapsed() > timeout {
            bail!("ComfyUI did not finish in {} s", timeout.as_secs());
        }
        let v: Value = http
            .get(format!("{base}/history/{id}"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let h = &v[id.as_str()];
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
        tokio::time::sleep(Duration::from_millis(500)).await;
    };

    tokio::fs::create_dir_all(out_dir).await?;
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
        if f.polls.fetch_add(1, Ordering::SeqCst) == 0 {
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
        });
        let app = Router::new()
            .route("/prompt", post(prompt))
            .route("/history/{id}", get(history))
            .route("/view", get(view))
            .route("/system_stats", get(stats))
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
}
