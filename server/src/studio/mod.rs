//! M6-05: saved ComfyUI workflows. Each `<workflows_dir>/<name>/` holds `workflow.toml`
//! (parameters mapped to node inputs, outputs, VRAM, models with licences) and the API-format
//! graph(s). A model licence that is not OSI/permissive shows as a warning; it never stops a run (LIC-01).
pub mod comfy;
pub mod licence;
pub mod workflow;
pub mod target;
pub mod team;
pub mod audio;
pub mod service;
pub mod facetags;
pub mod import;
pub mod to_assets;
pub mod watchdog;
pub mod remove;
pub mod negative;
pub mod helmet;

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
        .route("/studio/runs/{id}", get(run_get).delete(remove::delete_one))
        // STU-D1: delete finished runs (ids, or every failed one) with their files.
        .route("/studio/runs/delete", post(remove::delete_many))
        .route("/studio/target", get(target::read).put(target::set))
        // STU-01d: room for a face photo (12 MiB) in the multipart form.
        .route("/studio/make", post(team::make).layer(axum::extract::DefaultBodyLimit::max(60 * 1024 * 1024)))
        .route("/studio/audio", post(audio::make))
        .route("/studio/mine", get(team::mine))
        .route("/studio/runs/{id}/files/{n}", get(team::file))
        // STU-02b: a finished result becomes an asset of one of the user's projects.
        .route("/studio/runs/{id}/to-assets", post(to_assets::send))
        .route("/studio/target/service", get(target::service_read).put(target::service_set))
        // KS-01: Kreative Studio's jobs, on behalf of a studio user (auth::guard maps the user).
        .route("/studio/service/make", post(team::make).layer(axum::extract::DefaultBodyLimit::max(60 * 1024 * 1024)))
        .route("/studio/service/audio", post(audio::make))
        .route("/studio/service/workflows", get(list))
        .route("/studio/service/mine", get(team::mine))
        .route("/studio/service/runs/{id}", get(run_get).delete(remove::delete_one))
        .route("/studio/service/runs/delete", post(remove::delete_many))
        .route("/studio/service/runs/{id}/files/{n}", get(team::file))
        // KS-03: Kreative Studio's own finished jobs join the same library.
        .route("/studio/service/import", post(import::import))
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

        let problems = wf.licence_warnings();
        let runnable = true; // LIC-01: a licence only warns
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
            "face": wf.face.is_some(),
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

/// STU-02: when a job doesn't fit on `gpu`, free what idles there: ComfyUI's cache (unless the
/// job is ComfyUI's own) and the music/SFX apps other than `keep` (they hold 10-11.5 GB once loaded).
pub async fn make_room(s: &AppState, gpu: &str, need_mib: u64, keep: &str) {
    let free = crate::gpus::current(s).await.into_iter().find(|l| l.id == gpu).map(|l| l.free_mib);
    if free.is_some_and(|f| f >= need_mib) {
        return;
    }
    let Some(g) = s.config.gpus.iter().find(|g| g.id == gpu) else { return };
    for h in &g.holders {
        if let Some(url) = h.probe.strip_prefix("comfyui:") {
            if keep != "comfyui" {
                comfy::free_if_idle(&s.http, url.trim_end_matches('/')).await;
            }
        } else if let Some(url) = h.probe.strip_prefix("studio:")
            && h.app.as_deref() != Some(keep)
        {
            let url = url.trim_end_matches('/');
            let idle = match s.http.get(format!("{url}/health")).send().await {
                Ok(r) => r.json::<Value>().await.is_ok_and(|v| v["loaded"] == true && v["busy"] != true),
                Err(_) => false,
            };
            if idle {
                let _ = s.http.post(format!("{url}/unload")).send().await;
            }
        }
    }
}

pub fn make_room_while_waiting(s: &AppState, gpu: &str, need_mib: u64, keep: &'static str) -> tokio::task::JoinHandle<()> {
    let s = s.clone();
    let gpu = gpu.to_string();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(20)).await;
            let running: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gpu_job WHERE gpu = ? AND state = 'running'")
                .bind(&gpu)
                .fetch_one(&s.db)
                .await
                .unwrap_or(1);
            if running == 0 {
                crate::studio::make_room(&s, &gpu, need_mib, keep).await;
            }
        }
    })
}

/// GPU-02: the GPU a studio run uses. A named GPU is used as asked; "auto" takes the first
/// configured GPU with studio apps and a ComfyUI whose computer has the studio on, else
/// `[studio] fallback_gpu` (e.g. the A770, at the cost of Coder), else an error saying why.
pub async fn place(s: &AppState, gpu: &str, machines: &[String], has: impl Fn(&str) -> bool) -> Result<String, String> {
    // STU-02: a workflow may be limited to some machines (video: soucouyant).
    let allowed = |id: &str| machines.is_empty() || s.config.gpus.iter().any(|g| g.id == id && machines.contains(&g.machine));
    let only = || format!("This runs only on {}.", machines.join(", "));
    // GPU-03: "Studio runs on" (Off holds every job; a chosen GPU takes the automatic ones).
    let chosen = target::get(s).await;
    if chosen == "off" {
        return Err("The studio is off. Switch it on under Capabilities, GPUs (Studio runs on), or in Kreative Studio.".to_string());
    }
    if gpu != "auto" {
        return if allowed(gpu) { Ok(gpu.to_string()) } else { Err(only()) };
    }
    if chosen != "auto" {
        return if allowed(&chosen) { Ok(chosen) } else { Err(format!("{} The studio runs on {chosen} now.", only())) };
    }
    for g in &s.config.gpus {
        if g.apps.is_empty() || !has(&g.id) || !allowed(&g.id) {
            continue;
        }
        if crate::gpus::gaming::studio_allowed(s, &g.machine).await {
            return Ok(g.id.clone());
        }
    }
    if let Some(f) = &s.config.studio.fallback_gpu
        && has(f)
        && allowed(f)
    {
        return Ok(f.clone());
    }
    if !machines.is_empty() {
        return Err(format!("{} Its studio is off now.", only()));
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
    // STU-01c: adult rating guard
    if let Some(r) = wf.adult_choice(params) {
        if user_id != "cli" && !user_id.starts_with(service::STUDIO_PREFIX) && !crate::admin::is_adult(&s.db, user_id).await? {
            return Err(ApiError::Forbidden(format!("Your account may not use the {r} rating. An admin can allow adult content for it.")));
        }
    }
    let placed = place(s, gpu, &wf.machines, |id| comfy_target(&s.config.gpus, id).is_some()).await.map_err(ApiError::BadRequest)?;
    let gpu = placed.as_str();
    let (machine, url) = comfy_target(&s.config.gpus, gpu)
        .ok_or_else(|| ApiError::BadRequest(format!("{gpu} has no ComfyUI")))?;
    // GPU-01: refused while that computer games; starts ComfyUI there if Kompanion stopped it
    // (comfy::run waits for it to answer).
    crate::gpus::gaming::ensure_started(s, gpu, "comfyui").await.map_err(ApiError::BadRequest)?;
    // STU-01d: a face photo ("face": its file on this server, "face_weight") switches to the face graph.
    let mut params = params.clone();
    let face = params.remove("face").and_then(|v| v.as_str().map(std::path::PathBuf::from));
    let weight = params.remove("face_weight").and_then(|v| v.as_f64()).unwrap_or(0.85);
    // STU-R2: the user's pictures ("input:<name>": its file on this server) for the [inputs] LoadImage nodes.
    let mut pics = Vec::new();
    for (input, node) in &wf.inputs {
        let path = params.remove(&format!("input:{input}")).and_then(|v| v.as_str().map(std::path::PathBuf::from));
        match path {
            Some(p) => pics.push((node.clone(), p)),
            None => return Err(ApiError::BadRequest(format!("{} needs a picture ({input}).", wf.title))),
        }
    }
    let graph = match &face {
        Some(_) => wf.face_graph("", weight),
        None => wf.graph_for(&machine),
    }
    .map_err(ApiError::BadRequest)?;
    let (graph, mut used) = wf.fill(&graph, &params).map_err(ApiError::BadRequest)?;
    if face.is_some() {
        used.insert("face_weight".into(), json!(weight));
    }
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
            face,
            pics,
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
    mut graph: Value,
    used: Map<String, Value>,
    id: String,
    user_id: String,
    face: Option<std::path::PathBuf>,
    pics: Vec<(String, std::path::PathBuf)>,
) {
    // BUG-02: Studio off stops this run at once; it also has a time limit.
    let watch = watchdog::watch(&id, &gpu);
    let mut spec = jobs::Spec {
        kind: Kind::Asset,
        what: format!("studio:comfyui:{}", w.name),
        gpus: vec![gpu.clone()],
        vram_mib: w.vram_for(&machine),
        ram_mib: w.ram_for(&machine),
        tonight: false,
        run_id: Some(id.clone()),
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
    // STU-C3: the RAM ComfyUI already holds is used on the host and is part of what this run
    // needs; counting it twice kept oc-shots queued on soucouyant with 14 GB of VRAM free.
    if let Some(total) = jobs::ram_total_mib(&s, &machine).await
        && let Ok(r) = s.http.get(format!("{url}/system_stats")).timeout(Duration::from_secs(3)).send().await
        && let Ok(stats) = r.json::<Value>().await
    {
        spec.ram_mib = spec.ram_mib.saturating_sub(comfy::ram_held_mib(&stats, total));
    }
    // STU-02: idle music/SFX apps on this GPU make way too.
    make_room(&s, &gpu, spec.vram_mib, "comfyui").await;
    // STU-C3: a run queued behind another one frees ComfyUI's cache too, once that one is done.
    let nudge = make_room_while_waiting(&s, &gpu, spec.vram_mib, "");
    let leased = jobs::acquire(&s, spec, Duration::from_secs(1800)).await;
    nudge.abort();
    let result = match leased {
        Err(e) => Err(e),
        Ok(lease) => {
            // The output root is shared too (create_dir_all makes it 755 otherwise).
            let root = &s.config.studio.output_dir;
            if tokio::fs::create_dir_all(root).await.is_ok() {
                comfy::shared(root, 0o2775).await;
            }
            // STU-01: the team's runs go to a folder per user; CLI runs stay at the top.
            let out_dir = if user_id == "cli" { root.join(&w.name) } else { root.join("users").join(&user_id).join(&w.name) };
            // STU-01d: upload the face photo to that ComfyUI's input folder for the LoadImage node.
            let up: anyhow::Result<()> = match (&face, &w.face) {
                (Some(p), Some(f)) => match tokio::fs::read(p).await {
                    Ok(bytes) => {
                        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("png");
                        comfy::upload(&s.http, &url, bytes, &format!("{id}.{ext}")).await.map(|name| {
                            graph[f.image.as_str()]["inputs"]["image"] = json!(name);
                        })
                    }
                    Err(e) => Err(anyhow::anyhow!("face photo: {e}")),
                },
                _ => Ok(()),
            };
            // STU-C1: images shipped with the workflow (the OC sheet's pose and depth sheets).
            let up = match up {
                Err(e) => Err(e),
                Ok(()) => async {
                    // STU-R2: and the user's pictures for this run (a refine step's source and mask).
                    for (node, path) in &pics {
                        let bytes = tokio::fs::read(path).await.map_err(|e| anyhow::anyhow!("picture: {e}"))?;
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
                        let name = comfy::upload(&s.http, &url, bytes, &format!("{id}-{node}.{ext}")).await?;
                        graph[node.as_str()]["inputs"]["image"] = json!(name);
                    }
                    for (node, file) in &w.images {
                        let path = w.dir.join(file);
                        let bytes = tokio::fs::read(&path).await;
                        let name = match bytes {
                            Ok(b) => {
                                comfy::upload(&s.http, &url, b, &format!("{}-{file}", w.name)).await?
                            }
                            Err(e) => return Err(anyhow::anyhow!("{file}: {e}")),
                        };
                        graph[node.as_str()]["inputs"]["image"] = json!(name);
                    }
                    Ok(())
                }.await,
            };
            let r = match up {
                Err(e) => Err(e),
                Ok(()) => watchdog::guard(&watch, watchdog::limit("comfyui"), comfy::run(
                    &s.http,
                    &url,
                    &graph,
                    &w.outputs,
                    &out_dir,
                    &id,
                    Duration::from_secs(1800),
                ))
                .await,
            };
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
    // STU-01d: the photo leaves this server after the run (the CLI's own file stays).
    if let Some(p) = &face {
        if user_id != "cli" {
            let _ = tokio::fs::remove_file(p).await;
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
