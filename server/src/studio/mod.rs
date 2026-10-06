//! M6-05: saved ComfyUI workflows. Each `<workflows_dir>/<name>/` holds `workflow.toml`
//! (parameters mapped to node inputs, outputs, VRAM, models with licences) and the API-format
//! graph(s). Only workflows whose models all have an allowed licence can run.
pub mod comfy;
pub mod licence;
pub mod workflow;
pub mod target;
pub mod team;

use crate::{
    AppState,
    auth::User,
    config::GpuConfig,
    error::{ApiError, ApiResult},
    gpus::{jobs, sched::Kind},
    util,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::time::Duration;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/studio/workflows", get(list))
        .route("/studio/workflows/{name}/run", post(start))
        .route("/studio/runs/{id}", get(run_get))
        .route("/studio/target", get(target::read).put(target::set))
        .route("/studio/make", post(team::make))
        .route("/studio/mine", get(team::mine))
        .route("/studio/runs/{id}/files/{n}", get(team::file))
        .route("/studio/target/service", get(target::service_read).put(target::service_set))
}

pub fn comfy_target(gpus: &[GpuConfig], gpu: &str) -> Option<(String, String)> {
    let cfg = gpus.iter().find(|c| c.id == gpu)?;
    let holder = cfg
        .holders
        .iter()
        .find(|h| h.probe.starts_with("comfyui:"))?;
    Some((
        cfg.machine.clone(),
        holder
            .probe
            .strip_prefix("comfyui:")
            .unwrap()
            .trim_end_matches('/')
            .to_string(),
    ))
}

async fn list(State(s): State<AppState>) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(workflows_json(&s).await?))
}

/// Every workflow with its licences, targets and run numbers (the API and the Capabilities page).
pub async fn workflows_json(s: &AppState) -> ApiResult<Vec<Value>> {
    let mut rows: Vec<Value> = Vec::new();
    for (name, result) in workflow::load_all(&s.config.studio.workflows_dir) {
        let wf = match result {
            Ok(wf) => wf,
            Err(e) => {
                rows.push(json!({"name": name, "error": e, "runnable": false}));
                continue;
            }
        };

        let problems = wf.licence_problems();
        let runnable = problems.is_empty();
        let targets: Vec<String> = s
            .config
            .gpus
            .iter()
            .filter(|g| comfy_target(&s.config.gpus, &g.id).is_some())
            .map(|g| g.id.clone())
            .collect();
        let row = sqlx::query_as::<_, (i64, Option<String>, Option<f64>)>(
            "SELECT COUNT(*), MAX(started_at), AVG((julianday(ended_at) - julianday(started_at)) * 86400.0) FROM studio_run WHERE workflow = ? AND state = 'done'"
        )
        .bind(&name)
        .fetch_optional(&s.db)
        .await?;
        let (count, last, avg) = row.unwrap_or_default();
        rows.push(json!({
            "name": name,
            "title": wf.title,
            "description": wf.description,
            "studio": wf.studio,
            "base": wf.base,
            "params": wf.params,
            "models": wf.models,
            "vramMb": wf.vram_mb,
            "vramByMachine": wf.vram,
            "ramByMachine": wf.ram,
            "ramMb": wf.ram_mb,
            "problems": problems,
            "runnable": runnable,
            "targets": targets,
            "runs": count,
            "lastRun": last,
            "avgSeconds": avg
        }));
    }
    Ok(rows)
}

/// GPU-02: without a GPU, a run is placed by `place`.
fn auto_gpu() -> String {
    "auto".into()
}

#[derive(Deserialize)]
struct Start {
    #[serde(default = "auto_gpu")]
    gpu: String,
    #[serde(default)]
    params: Map<String, Value>,
}

async fn start(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(name): Path<String>,
    Json(b): Json<Start>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    if !crate::admin::is_admin(&s.db, &u.id).await? {
        return Err(ApiError::Forbidden("Only admins can run workflows.".into()));
    }
    let (id, job) = queue(&s, &name, &b.gpu, &b.params, &u.id).await?;
    tokio::spawn(job);
    Ok((StatusCode::ACCEPTED, Json(json!({"id": id}))))
}

/// GPU-02: the GPU a studio run uses. A named GPU is used as asked; "auto" takes the first
/// configured GPU with studio apps and a ComfyUI whose computer has the studio on, else
/// `[studio] fallback_gpu` (e.g. the A770, at the cost of Coder), else an error saying why.
pub async fn place(s: &AppState, gpu: &str) -> Result<String, String> {
    // GPU-03: "Studio runs on" (Off holds every job; a chosen GPU takes the automatic ones).
    let chosen = target::get(s).await;
    if chosen == "off" {
        return Err("The studio is off. Switch it on under Capabilities, GPUs (Studio runs on), or in Kreative Studio.".to_string());
    }
    if gpu != "auto" {
        return Ok(gpu.to_string());
    }
    if chosen != "auto" {
        return Ok(chosen);
    }
    for g in &s.config.gpus {
        if g.apps.is_empty() || comfy_target(&s.config.gpus, &g.id).is_none() {
            continue;
        }
        if crate::gpus::gaming::studio_allowed(s, &g.machine).await {
            return Ok(g.id.clone());
        }
    }
    if let Some(f) = &s.config.studio.fallback_gpu
        && comfy_target(&s.config.gpus, f).is_some()
    {
        return Ok(f.clone());
    }
    Err("The studio is off on every studio computer and no other GPU may take studio jobs (kompanion.toml [studio] fallback_gpu).".to_string())
}

/// Check and record a run; the returned future runs it (licences, GPU and parameters are
/// checked here, so a refused workflow never queues).
async fn queue(
    s: &AppState,
    name: &str,
    gpu: &str,
    params: &Map<String, Value>,
    user_id: &str,
) -> ApiResult<(
    String,
    impl std::future::Future<Output = ()> + Send + 'static,
)> {
    let wf = match workflow::load_all(&s.config.studio.workflows_dir)
        .into_iter()
        .find(|(n, _)| n == name)
    {
        None => return Err(ApiError::NotFound),
        Some((_, Err(e))) => return Err(ApiError::BadRequest(e)),
        Some((_, Ok(w))) => w,
    };
    let problems = wf.licence_problems();
    if !problems.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "Refused licences: {}",
            problems.join("; ")
        )));
    }
    let placed = place(s, gpu).await.map_err(ApiError::BadRequest)?;
    let gpu = placed.as_str();
    let (machine, url) = comfy_target(&s.config.gpus, gpu)
        .ok_or_else(|| ApiError::BadRequest(format!("{gpu} has no ComfyUI")))?;
    // GPU-01: refused while that computer games; starts ComfyUI there if Kompanion stopped it
    // (comfy::run waits for it to answer).
    crate::gpus::gaming::ensure_started(s, gpu, "comfyui").await.map_err(ApiError::BadRequest)?;
    let graph = wf.graph_for(&machine).map_err(ApiError::BadRequest)?;
    let (graph, used) = wf.fill(&graph, params).map_err(ApiError::BadRequest)?;
    let id = util::new_id();
    sqlx::query("INSERT INTO studio_run (id, workflow, gpu, user_id, params, models, state, started_at) VALUES (?, ?, ?, ?, ?, ?, 'running', ?)")
        .bind(&id)
        .bind(name)
        .bind(gpu)
        .bind(user_id)
        .bind(Value::Object(used.clone()).to_string())
        .bind(serde_json::to_string(&wf.models).unwrap_or_default())
        .bind(util::now())
        .execute(&s.db)
        .await
        .map_err(anyhow::Error::from)?;
    Ok((
        id.clone(),
        execute(
            s.clone(),
            wf,
            gpu.to_string(),
            machine,
            url,
            graph,
            used,
            id,
            user_id.to_string(),
        ),
    ))
}

/// `kompanion-server studio-run <workflow> <gpu> [params as JSON]`: queue a run as a normal GPU
/// job (the running server's scheduler places it), wait for it and print the run.
pub async fn cli(s: &AppState, args: &[String]) -> anyhow::Result<()> {
    let (Some(name), Some(gpu)) = (args.first(), args.get(1)) else {
        anyhow::bail!("usage: kompanion-server studio-run <workflow> <gpu> [params JSON]");
    };
    let params: Map<String, Value> = match args.get(2) {
        Some(p) => serde_json::from_str(p)?,
        None => Map::new(),
    };
    let (id, job) = queue(s, name, gpu, &params, "cli")
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("run {id} queued on {gpu}");
    job.await;
    let Json(run) = run_get(State(s.clone()), Path(id))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", serde_json::to_string_pretty(&run)?);
    Ok(())
}

async fn execute(
    s: AppState,
    w: workflow::Workflow,
    gpu: String,
    machine: String,
    url: String,
    graph: Value,
    used: Map<String, Value>,
    id: String,
    user_id: String,
) {
    let mut spec = jobs::Spec {
        kind: Kind::Asset,
        what: format!("studio:comfyui:{}", w.name),
        gpus: vec![gpu.clone()],
        vram_mib: w.vram_for(&machine),
        ram_mib: w.ram_for(&machine),
        tonight: false,
    };
    // GPU-02: the same workflow again reuses ComfyUI's loaded models (16 s instead of 68 s on
    // the A770): ask only for what is missing. Another workflow frees the cache when it doesn't fit.
    let ledger = crate::gpus::current(&s).await.into_iter().find(|l| l.id == gpu);
    let cached: u64 = ledger.as_ref()
        .map(|l| l.holdings.iter().filter(|h| h.kind == "app" && !h.busy && h.name.starts_with("ComfyUI")).map(|h| h.now_mib).sum())
        .unwrap_or(0);
    let last: Option<String> = sqlx::query_scalar(
        "SELECT workflow FROM studio_run WHERE gpu = ? AND state = 'done' ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&gpu)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if cached > 0 && last.as_deref() == Some(w.name.as_str()) {
        spec.vram_mib = spec.vram_mib.saturating_sub(cached);
    } else if ledger.as_ref().is_none_or(|l| l.free_mib < spec.vram_mib) {
        comfy::free_if_idle(&s.http, &url).await;
    }
    let result = match jobs::acquire(&s, spec, Duration::from_secs(3600)).await {
        Err(e) => Err(e),
        Ok(lease) => {
            // The output root is shared too (create_dir_all makes it 755 otherwise).
            let root = &s.config.studio.output_dir;
            if tokio::fs::create_dir_all(root).await.is_ok() {
                comfy::shared(root, 0o2775).await;
            }
            // STU-01: the team's runs go to a folder per user; CLI runs stay at the top.
            let out_dir = if user_id == "cli" { root.join(&w.name) } else { root.join("users").join(&user_id).join(&w.name) };
            let r = comfy::run(
                &s.http,
                &url,
                &graph,
                &w.outputs,
                &out_dir,
                &id,
                Duration::from_secs(1800),
            )
            .await;
            if let Err(e) = &r {
                lease.fail(e.to_string());
            }
            r
        }
    };
    match result {
        Ok(files) => {
            for f in &files {
                if let Ok(bytes) = serde_json::to_vec_pretty(
                    &json!({"workflow": w.name, "title": w.title, "run": id, "gpu": gpu, "machine": machine, "params": used, "models": w.models, "created": util::now()}),
                ) {
                    let side = std::path::PathBuf::from(format!("{}.json", f.display()));
                    if tokio::fs::write(&side, bytes).await.is_ok() {
                        comfy::shared(&side, 0o664).await;
                    }
                }
            }
            let outputs = json!(
                files
                    .iter()
                    .map(|f| f.display().to_string())
                    .collect::<Vec<_>>()
            )
            .to_string();
            let _ = sqlx::query(
                "UPDATE studio_run SET state = 'done', outputs = ?, ended_at = ? WHERE id = ?",
            )
            .bind(outputs)
            .bind(util::now())
            .bind(id)
            .execute(&s.db)
            .await;
        }
        Err(e) => {
            tracing::warn!("workflow {} failed: {e:#}", w.name);
            let _ = sqlx::query(
                "UPDATE studio_run SET state = 'failed', error = ?, ended_at = ? WHERE id = ?",
            )
            .bind(e.to_string())
            .bind(util::now())
            .bind(id)
            .execute(&s.db)
            .await;
        }
    }
    // STU-01: the user's Studio queue and library update live.
    s.bus.send(&user_id, crate::events::Event::Changed { what: "studio", machine_id: None });
}

async fn run_get(State(s): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, String, String, Option<String>)>(
        "SELECT workflow, gpu, params, models, state, error, outputs, started_at, ended_at FROM studio_run WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await.map_err(anyhow::Error::from)?;
    match row {
        None => Err(ApiError::NotFound),
        Some((wf, gpu, params, models, state, error, outputs, started, ended)) => {
            let params_val = serde_json::from_str::<Value>(&params).unwrap_or(Value::Null);
            let models_val = serde_json::from_str::<Value>(&models).unwrap_or(Value::Null);
            let outputs_val = serde_json::from_str::<Value>(&outputs).unwrap_or(Value::Null);
            Ok(Json(json!({
                "id": id,
                "workflow": wf,
                "gpu": gpu,
                "params": params_val,
                "models": models_val,
                "state": state,
                "error": error,
                "outputs": outputs_val,
                "startedAt": started,
                "endedAt": ended
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target() {
        let gpus: Vec<GpuConfig> = toml::from_str::<std::collections::HashMap<String, Vec<GpuConfig>>>(
            "[[gpu]]\nid = \"a770\"\nmachine = \"kireserver\"\npci = \"x\"\nvram_gb = 16\n[[gpu.holder]]\nname = \"Coder\"\nprobe = \"ovms:http://ovms:8000\"\n[[gpu.holder]]\nname = \"ComfyUI\"\nprobe = \"comfyui:http://comfyui:188/\"\n[[gpu]]\nid = \"a580\"\nmachine = \"kireserver\"\npci = \"y\"\nvram_gb = 8\n"
        ).unwrap().remove("gpu").unwrap();
        assert_eq!(
            comfy_target(&gpus, "a770"),
            Some(("kireserver".to_string(), "http://comfyui:188".to_string()))
        );
        assert_eq!(comfy_target(&gpus, "a580"), None);
        assert_eq!(comfy_target(&gpus, "nope"), None);
    }
}
