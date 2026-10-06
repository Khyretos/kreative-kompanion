//! M6-05: saved ComfyUI workflows. Each `<workflows_dir>/<name>/` holds `workflow.toml`
//! (parameters mapped to node inputs, outputs, VRAM, models with licences) and the API-format
//! graph(s). Only workflows whose models all have an allowed licence can run.
pub mod comfy;
pub mod licence;
pub mod workflow;

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
    Ok(Json(rows))
}

#[derive(Deserialize)]
struct Start {
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
    let (machine, url) = comfy_target(&s.config.gpus, gpu)
        .ok_or_else(|| ApiError::BadRequest(format!("{gpu} has no ComfyUI")))?;
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
) {
    let spec = jobs::Spec {
        kind: Kind::Asset,
        what: format!("studio:comfyui:{}", w.name),
        gpus: vec![gpu.clone()],
        vram_mib: w.vram_for(&machine),
        ram_mib: w.ram_for(&machine),
        tonight: false,
    };
    // GPU-01: keep the models loaded while jobs keep coming (~9 s per image instead of ~19 s);
    // free the target ComfyUI's cache only when the ledger says the job doesn't fit next to it.
    let free = crate::gpus::current(&s).await.into_iter().find(|l| l.id == gpu).map(|l| l.free_mib);
    if free.is_none_or(|f| f < spec.vram_mib) {
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
            let out_dir = root.join(&w.name);
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
