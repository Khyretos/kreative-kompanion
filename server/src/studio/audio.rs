//! STU-02: music (HeartMuLa) and sound effects (MOSS) from the Studio, through the same placement,
//! GPU scheduler and per-user folders as images.
use std::time::Duration;
use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use crate::{AppState, auth::User, config::GpuConfig, error::{ApiError, ApiResult}, gpus::{jobs, sched::Kind}, util};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioKind {
    pub kind: &'static str,
    pub app: &'static str,
    pub runner_app: &'static str,
    pub path: &'static str,
    pub min_s: u32,
    pub max_s: u32,
    pub default_s: u32,
    pub vram_mib: u64,
}

pub fn kind_of(kind: &str) -> Option<AudioKind> {
    match kind {
        "music" => Some(AudioKind {
            kind: "music",
            app: "heartmula",
            runner_app: "heartmula",
            path: "/music",
            min_s: 5,
            max_s: 240,
            default_s: 30,
            vram_mib: 10000,
        }),
        "sfx" => Some(AudioKind {
            kind: "sfx",
            app: "moss-sfx",
            runner_app: "sfx",
            path: "/sfx",
            min_s: 1,
            max_s: 30,
            default_s: 4,
            vram_mib: 11500,
        }),
        _ => None,
    }
}

pub fn audio_target(gpus: &[GpuConfig], gpu: &str, app: &str) -> Option<(String, String)> {
    let g = gpus.iter().find(|g| g.id == gpu)?;
    let h = g.holders.iter().find(|h| h.app.as_deref() == Some(app) && h.probe.starts_with("studio:"))?;
    Some((g.machine.clone(), h.probe.trim_start_matches("studio:").trim_end_matches('/').to_string()))
}

pub fn request_body(k: AudioKind, prompt: &str, lyrics: &str, seconds: u32, seed: u64) -> Value {
    if k.kind == "music" {
        json!({
            "tags": prompt,
            "lyrics": lyrics,
            "seconds": seconds,
            "seed": seed
        })
    } else {
        json!({
            "prompt": prompt,
            "seconds": seconds,
            "seed": seed
        })
    }
}

#[derive(Deserialize)]
pub struct Make {
    kind: String,
    prompt: String,
    #[serde(default)]
    lyrics: String,
    #[serde(default)]
    seconds: Option<u32>,
}

pub async fn make(State(s): State<AppState>, Extension(u): Extension<User>, Json(b): Json<Make>) -> ApiResult<(StatusCode, Json<Value>)> {
    let (id, job) = begin(&s, b, &u.id).await?;
    tokio::spawn(job);
    Ok((StatusCode::ACCEPTED, Json(json!({"ids": [id]}))))
}

/// `kompanion-server studio-audio <music|sfx> <description> [seconds]`: one audio job through the
/// running server's scheduler, waited for and printed (like studio-run).
pub async fn cli(s: &AppState, args: &[String]) -> anyhow::Result<()> {
    let (Some(kind), Some(prompt)) = (args.first(), args.get(1)) else {
        anyhow::bail!("usage: kompanion-server studio-audio <music|sfx> <description> [seconds]");
    };
    let b = Make { kind: kind.clone(), prompt: prompt.clone(), lyrics: String::new(), seconds: args.get(2).and_then(|x| x.parse().ok()) };
    let (id, job) = begin(s, b, "cli").await.map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("run {id} queued");
    job.await;
    let row: (String, Option<String>, String) = sqlx::query_as("SELECT state, error, outputs FROM studio_run WHERE id = ?").bind(&id).fetch_one(&s.db).await?;
    println!("{}", json!({"id": id, "state": row.0, "error": row.1, "outputs": serde_json::from_str::<Value>(&row.2).unwrap_or_default()}));
    Ok(())
}

/// Check and record an audio job; the returned future runs it.
async fn begin(s: &AppState, b: Make, user_id: &str) -> ApiResult<(String, impl std::future::Future<Output = ()> + Send + 'static)> {
    let k = kind_of(&b.kind).ok_or_else(|| ApiError::BadRequest("Pick music or a sound effect.".into()))?;
    
    let prompt = b.prompt.trim();
    if prompt.is_empty() || prompt.len() > 1000 {
        return Err(ApiError::BadRequest("Describe the sound (up to 1000 characters).".into()));
    }
    
    if b.lyrics.len() > 3000 {
        return Err(ApiError::BadRequest("Lyrics can be at most 3000 characters.".into()));
    }
    
    let seconds = b.seconds.unwrap_or(k.default_s).clamp(k.min_s, k.max_s);
    let lyrics = if b.lyrics.trim().is_empty() { "[Instrumental]".to_string() } else { b.lyrics.clone() };
    let seed: u64 = rand::random::<u32>() as u64;
    
    let gpu = super::place(&s, "auto", &[], |id| audio_target(&s.config.gpus, id, k.app).is_some()).await.map_err(ApiError::BadRequest)?;
    let (machine, url) = audio_target(&s.config.gpus, &gpu, k.app).ok_or_else(|| ApiError::BadRequest(format!("{gpu} has no {}", k.app)))?;
    
    crate::gpus::gaming::ensure_started(&s, &gpu, k.runner_app).await.map_err(ApiError::BadRequest)?;
    
    let id = util::new_id();
    let params = json!({"prompt": prompt, "lyrics": if k.kind == "music" { json!(lyrics) } else { Value::Null }, "seconds": seconds, "seed": seed});
    let models = json!([{"file": if k.kind == "music" { "HeartMuLa" } else { "MOSS-SoundEffect" }, "licence": "Apache-2.0"}]);
    

    
    sqlx::query(
        r#"INSERT INTO studio_run (id, workflow, gpu, user_id, params, models, state, started_at) VALUES (?, ?, ?, ?, ?, ?, 'running', ?)"#
    )
    .bind(id.clone())
    .bind(k.kind)
    .bind(&gpu)
    .bind(user_id)
    .bind(params.to_string())
    .bind(models.to_string())
    .bind(util::now())
    .execute(&s.db)
    .await.map_err(ApiError::from)?;
    
    let job = run(s.clone(), k, gpu, url, params, id.clone(), user_id.to_string());
    Ok((id, job))
}

/// STU-A1: the runner starts a stopped audio app with docker start, which takes seconds to listen;
/// a job sent at once failed with "connection refused". Waits until GET {url}/health answers at all.
pub async fn wait_up(http: &reqwest::Client, url: &str, within: Duration) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    loop {
        if http.get(format!("{url}/health")).timeout(Duration::from_secs(5)).send().await.is_ok() {
            return Ok(());
        }
        if start.elapsed() >= within {
            anyhow::bail!("{url} did not start within {} s", within.as_secs());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// BUG-02: POST the job to the audio app and wait for its answer; fails when its /health has not
/// shown it busy for `stall` (polled every stall / 9), or when it answers with an error.
pub async fn request(http: &reqwest::Client, url: &str, k: AudioKind, body: &Value, stall: Duration) -> anyhow::Result<Map<String, Value>> {
    let post = http.post(format!("{url}{}", k.path)).json(body).timeout(super::watchdog::limit(k.kind)).send();
    tokio::pin!(post);
    let mut progress = super::watchdog::Progress::new(stall, std::time::Instant::now());
    let r = loop {
        tokio::select! {
            r = &mut post => break r?,
            _ = tokio::time::sleep(stall / 9) => {
                let health = http.get(format!("{url}/health")).timeout(Duration::from_secs(5)).send().await;
                let alive = match health {
                    Ok(h) => h.json::<Value>().await.is_ok_and(|v| super::watchdog::audio_alive(&v)),
                    Err(_) => false,
                };
                progress.tick(alive, std::time::Instant::now())?;
            }
        }
    };
    if !r.status().is_success() {
        anyhow::bail!("{} answered {}", k.app, r.status());
    }
    Ok(r.json().await?)
}

async fn run(s: AppState, k: AudioKind, gpu: String, url: String, params: Value, id: String, user_id: String) {
    let w = super::watchdog::watch(&id, &gpu);
    let spec = jobs::Spec {
        kind: Kind::Asset,
        what: format!("studio:{}:{}", k.app, k.kind),
        gpus: vec![gpu.clone()],
        vram_mib: k.vram_mib,
        ram_mib: 6000,
        tonight: false,
        run_id: Some(id.clone()),
    };
    
    let result: anyhow::Result<Vec<String>> = async {
        super::make_room(&s, &gpu, k.vram_mib, k.app).await;
        let nudge = super::make_room_while_waiting(&s, &gpu, k.vram_mib, k.app);
        let leased = jobs::acquire(&s, spec, Duration::from_secs(900)).await;
        nudge.abort();
        let lease = leased?;
        if let Err(e) = wait_up(&s.http, &url, Duration::from_secs(120)).await {
            lease.fail(e.to_string());
            return Err(e);
        }
        
        let body = request_body(
            k,
            params["prompt"].as_str().unwrap_or(""),
            params["lyrics"].as_str().unwrap_or("[Instrumental]"),
            params["seconds"].as_u64().unwrap_or(k.default_s as u64) as u32,
            params["seed"].as_u64().unwrap_or(0),
        );
        
        let answer = match super::watchdog::guard(&w, super::watchdog::limit(k.kind), request(&s.http, &url, k, &body, super::watchdog::STALL)).await {
            Ok(a) => a,
            Err(e) => {
                lease.fail(e.to_string());
                return Err(e);
            }
        };
        
        let dir = s.config.studio.output_dir.join("users").join(&user_id).join(k.kind);
        tokio::fs::create_dir_all(&dir).await?;
        
        let mut paths = Vec::new();
        for (key, ext) in [("file", "ogg"), ("wav", "wav")] {
            let Some(name) = answer.get(key).and_then(Value::as_str) else { continue };
            let bytes = s.http.get(format!("{url}/files/{name}")).send().await?.error_for_status()?.bytes().await?;
            let path = dir.join(format!("{id}.{ext}"));
            tokio::fs::write(&path, &bytes).await?;
            super::comfy::shared(&path, 0o664).await;
            paths.push(path.to_string_lossy().to_string());
        }
        let Some(first) = paths.first() else {
            lease.fail("no output");
            anyhow::bail!("{} answered without a file", k.app);
        };
        // Provenance next to the .ogg, as for images.
        let provenance = json!({"workflow": k.kind, "run": id, "gpu": gpu, "params": params, "created": util::now()});
        let side = format!("{first}.json");
        if tokio::fs::write(&side, serde_json::to_vec_pretty(&provenance)?).await.is_ok() {
            super::comfy::shared(std::path::Path::new(&side), 0o664).await;
        }
        drop(lease);
        Ok(paths)
    }.await;
    
    let (state, outputs, error) = match result {
        Ok(paths) => ("done", json!(paths).to_string(), None),
        Err(e) => {
            tracing::warn!("audio job {id} failed: {e:#}");
            ("failed", "[]".to_string(), Some(e.to_string()))
        }
    };
    let _ = sqlx::query("UPDATE studio_run SET state = ?, outputs = ?, error = ?, ended_at = ? WHERE id = ?")
        .bind(state)
        .bind(outputs)
        .bind(error)
        .bind(util::now())
        .bind(&id)
        .execute(&s.db)
        .await;
    s.bus.send(&user_id, crate::events::Event::Changed { what: "studio", machine_id: None });
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn kinds() {
        let m = kind_of("music").unwrap();
        assert_eq!((m.app, m.runner_app, m.path, m.max_s), ("heartmula", "heartmula", "/music", 240));
        
        let x = kind_of("sfx").unwrap();
        assert_eq!((x.app, x.runner_app, x.path, x.default_s), ("moss-sfx", "sfx", "/sfx", 4));
        
        assert!(kind_of("video").is_none());
    }
    
    #[test]
    fn bodies() {
        let music_k = kind_of("music").unwrap();
        assert_eq!(request_body(music_k, "lofi", "[Instrumental]", 30, 7), json!({"tags": "lofi", "lyrics": "[Instrumental]", "seconds": 30, "seed": 7}));
        
        let sfx_k = kind_of("sfx").unwrap();
        assert_eq!(request_body(sfx_k, "door creak", "x", 4, 9), json!({"prompt": "door creak", "seconds": 4, "seed": 9}));
    }
    
    #[test]
    fn targets() {
        #[derive(serde::Deserialize)]
        struct G { gpu: Vec<GpuConfig> }
        
        let toml_str = r#"
        [[gpu]]
        id = "a770"
        machine = "kireserver"
        pci = "x"
        vram_gb = 16
        [[gpu.holder]]
        name = "HeartMuLa"
        kind = "app"
        probe = "studio:http://heartmula:8190/"
        app = "heartmula"
        [[gpu]]
        id = "a580"
        machine = "kireserver"
        pci = "y"
        vram_gb = 8
        "#;
        
        let g: G = toml::from_str(toml_str).unwrap();
        let gpus = g.gpu;
        
        assert_eq!(audio_target(&gpus, "a770", "heartmula"), Some(("kireserver".to_string(), "http://heartmula:8190".to_string())));
        assert_eq!(audio_target(&gpus, "a770", "moss-sfx"), None);
        assert_eq!(audio_target(&gpus, "a580", "heartmula"), None);
    }

    /// STU-A1: the runner starts a stopped app with `docker start`; it takes seconds to listen.
    #[tokio::test]
    async fn waits_for_an_app_that_starts_late() {
        use axum::{Router, routing::get};
        let addr = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let app = Router::new().route("/health", get(|| async { Json(json!({"loaded": false, "busy": false})) }));
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });
        wait_up(&reqwest::Client::new(), &format!("http://{addr}"), Duration::from_secs(10)).await.unwrap();
    }

    #[tokio::test]
    async fn gives_up_on_an_app_that_never_starts() {
        let addr = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        let t = std::time::Instant::now();
        let e = wait_up(&reqwest::Client::new(), &format!("http://{addr}"), Duration::from_millis(1500)).await.unwrap_err().to_string();
        assert!(e.contains("did not start within 1 s"), "{e}");
        assert!(t.elapsed() < Duration::from_secs(5));
    }

    /// BUG-02: a fake audio app: /sfx answers after `wait_ms` with `status`, /health says `busy`.
    async fn fake_app(wait_ms: u64, status: u16, busy: bool) -> String {
        use axum::{Router, routing::{get, post}};
        let app = Router::new()
            .route("/sfx", post(move || async move {
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                (axum::http::StatusCode::from_u16(status).unwrap(), Json(json!({"file": "a.ogg"})))
            }))
            .route("/health", get(move || async move { Json(json!({"loaded": true, "busy": busy})) }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(axum::serve(listener, app).into_future());
        url
    }

    #[tokio::test]
    async fn a_busy_app_answers() {
        let url = fake_app(300, 200, true).await;
        let k = kind_of("sfx").unwrap();
        let a = request(&reqwest::Client::new(), &url, k, &json!({}), Duration::from_millis(900)).await.unwrap();
        assert_eq!(a.get("file").and_then(Value::as_str), Some("a.ogg"));
    }

    #[tokio::test]
    async fn a_stuck_app_fails_after_the_stall() {
        let url = fake_app(10_000, 200, false).await;
        let k = kind_of("sfx").unwrap();
        let t = std::time::Instant::now();
        let e = request(&reqwest::Client::new(), &url, k, &json!({}), Duration::from_millis(900)).await.unwrap_err().to_string();
        assert!(e.contains("no progress"), "{e}");
        assert!(t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
    }

    #[tokio::test]
    async fn an_app_error_is_reported() {
        let url = fake_app(0, 500, true).await;
        let k = kind_of("sfx").unwrap();
        let e = request(&reqwest::Client::new(), &url, k, &json!({}), Duration::from_millis(900)).await.unwrap_err().to_string();
        assert!(e.contains("moss-sfx answered 500"), "{e}");
    }
}
