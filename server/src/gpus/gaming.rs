//! GPU-01: Studio / Gaming / Auto per computer with studio apps; the runner's `gpu_apps` tool
//! stops and starts them. Decisions come from gaming_policy.
use std::{collections::{HashMap, HashSet}, sync::{LazyLock, Mutex}, time::{Duration, Instant}};
use axum::{Extension, Json, extract::{Path, State}};
use serde::Deserialize;
use serde_json::{Value, json};
use super::{gaming_policy::{self, Action, GpuMode}, ledger::GpuLedger};
use crate::{AppState, auth::User, error::{ApiError, ApiResult}, events::Event, util};

#[derive(Default)]
struct Seen { 
    was_gaming: bool, 
    sent_at: Option<Instant> 
}

static SEEN: LazyLock<Mutex<HashMap<String, Seen>>> = LazyLock::new(Default::default);

#[derive(Clone, Debug)]
struct Machine { 
    id: String, 
    user_id: String, 
    name: String, 
    mode: GpuMode, 
    apps_stopped: bool, 
    studio_at: Option<String>,
    /// The runner has the "gpu" system grant; without it nothing is stopped or started.
    granted: bool,
    /// GPU-04: a game ran here until this time plus the cooldown (RFC 3339, from the database).
    gaming_until: Option<String>,
}

async fn machines(s: &AppState) -> Vec<Machine> {
    let mut result: Vec<Machine> = Vec::new();
    for g in &s.config.gpus {
        if g.apps.is_empty() || result.iter().any(|m| m.name == g.machine) { continue; }
        let row = sqlx::query_as::<_, (String, String, String, i64, Option<String>, bool, Option<String>)>(
            "SELECT id, user_id, gpu_mode, apps_stopped, studio_at,
                    EXISTS(SELECT 1 FROM machine_grants g WHERE g.machine_id = machines.id AND g.target = 'system'
                           AND g.rights LIKE '%\"gpu\"%' AND (g.expires IS NULL OR g.expires > ?)),
                    gaming_until
             FROM machines WHERE name = ?"
        )
        .bind(util::now())
        .bind(&g.machine)
        .fetch_optional(&s.db)
        .await;
        
        match row {
            Ok(Some(r)) => {
                let mode = GpuMode::parse(&r.2).unwrap_or(GpuMode::Auto);
                let apps_stopped = r.3 != 0;
                result.push(Machine {
                    id: r.0,
                    user_id: r.1,
                    name: g.machine.clone(),
                    mode,
                    apps_stopped,
                    studio_at: r.4,
                    granted: r.5,
                    gaming_until: r.6,
                });
            }
            Ok(None) | Err(_) => {}
        }
    }
    result
}

/// GPU-04: the effective mode and whether a game runs or ran within the cooldown. The server
/// writes gaming_until (now + the cooldown) while a game runs and when it ends, so the CLI,
/// which gets no runner reports, reads the same state from the database.
async fn effective_of(s: &AppState, m: &Machine) -> (GpuMode, bool) {
    let live = s.host.gaming(&m.id).unwrap_or(false);

    let ended = {
        let mut map = SEEN.lock().unwrap();
        let e = map.entry(m.id.clone()).or_default();
        let ended = e.was_gaming && !live;
        e.was_gaming = live;
        ended
    };

    let until = if live || ended {
        let u = util::in_minutes(gaming_policy::GAME_COOLDOWN.as_secs() as i64 / 60);
        let _ = sqlx::query("UPDATE machines SET gaming_until = ? WHERE id = ?")
            .bind(&u)
            .bind(&m.id)
            .execute(&s.db)
            .await;
        Some(u)
    } else {
        m.gaming_until.clone()
    };

    let cooling = until.as_deref().is_some_and(|u| u > util::now().as_str());

    (gaming_policy::effective(m.mode, live || cooling, None), live || cooling)
}

/// GPU-04: the computers whose studio Kompanion manages ("gpu" grant and studio apps), set by step().
static MANAGED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

/// GPU-04: until when a computer reports fast (a mode change or a stop is pending).
static FAST: LazyLock<Mutex<HashMap<String, Instant>>> = LazyLock::new(Default::default);

/// GPU-04: report every 3 s for the next 2 minutes, so a stop reaches the runner at once.
fn hurry(machine_id: &str) {
    let mut map = FAST.lock().unwrap();
    map.insert(
        machine_id.to_string(),
        Instant::now() + Duration::from_secs(120),
    );
}

/// GPU-04: how often a runner reports: every 3 s while a change is pending, at most every 5 s on a managed computer (BUG-02: Studio off reaches it within seconds), else as asked.
pub fn report_interval(machine_id: &str, interval: u32) -> u32 {
    if let Some(exp) = FAST.lock().unwrap().get(machine_id) {
        if *exp > Instant::now() {
            return interval.min(3);
        }
    }
    if MANAGED.lock().unwrap().contains(machine_id) {
        return interval.min(5);
    }
    interval
}


async fn send(s: &AppState, m: &Machine, tool: Value) {
    if let Err(e) = crate::access::queue_job(&s.db, &m.id, &m.user_id, &tool, None).await {
        tracing::warn!(machine = %m.name, "GPU-01: can't queue {tool}: {e}");
    }
}

/// BUG-02: Studio off is off: kill the apps, unload Ollama and end this computer's studio jobs now.
async fn off_now(s: &AppState, m: &Machine) {
    send(s, m, json!({"tool": "gpu_apps", "action": "kill"})).await;
    send(s, m, json!({"tool": "gpu_apps", "action": "unload_ollama"})).await;
    let _ = sqlx::query("UPDATE machines SET apps_stopped = 1 WHERE id = ?").bind(&m.id).execute(&s.db).await;
    let gpus: Vec<String> = s.config.gpus.iter().filter(|g| g.machine == m.name).map(|g| g.id.clone()).collect();
    for g in &gpus {
        let _ = sqlx::query(
            "UPDATE gpu_job SET state = 'dropped', error = 'Studio turned off', ended_at = ?
             WHERE state IN ('queued', 'running') AND what LIKE 'studio:%' AND (gpu = ? OR (gpu IS NULL AND gpus LIKE ?))",
        )
        .bind(util::now())
        .bind(g)
        .bind(format!("%\"{g}\"%"))
        .execute(&s.db)
        .await;
    }
    let stopped = crate::studio::watchdog::stop_gpus(&gpus);
    SEEN.lock().unwrap().entry(m.id.clone()).or_default().sent_at = Some(Instant::now());
    hurry(&m.id);
    super::jobs::KICK.notify_one();
    tracing::info!(machine = %m.name, stopped, "BUG-02: Studio off, apps killed");
}

pub async fn step(s: &AppState, ledgers: &[GpuLedger]) {
    let list = machines(s).await;
    *MANAGED.lock().unwrap() = list.iter().filter(|m| m.granted).map(|m| m.id.clone()).collect();
    for m in list.iter().filter(|m| m.granted) {
        let (eff, _) = effective_of(s, m).await;
        
        let app_seen = ledgers.iter()
            .filter(|l| l.machine == m.name)
            .any(|l| l.holdings.iter().any(|h| h.kind == "app" && h.now_mib > 0));
        
        let ollama_loaded = ledgers.iter()
            .filter(|l| l.machine == m.name)
            .any(|l| l.holdings.iter().any(|h| h.kind == "model" && h.now_mib > 0));
        
        let busy = ledgers.iter()
            .filter(|l| l.machine == m.name)
            .any(|l| l.holdings.iter().any(|h| h.busy));
        
        if m.apps_stopped && app_seen {
            let _ = sqlx::query("UPDATE machines SET apps_stopped = 0 WHERE id = ?")
                .bind(&m.id)
                .execute(&s.db)
                .await;
        }
        
        let apps_up = !m.apps_stopped || app_seen;
        // No studio job seen yet (just granted, or a fresh install): the idle clock starts now,
        // so granting the right doesn't stop the apps at once (GPU-03).
        if m.studio_at.is_none() {
            let _ = sqlx::query("UPDATE machines SET studio_at = ? WHERE id = ?").bind(util::now()).bind(&m.id).execute(&s.db).await;
        }
        let idle_long = !busy && m.studio_at.as_deref().is_some_and(|t| t < util::minutes_ago(gaming_policy::IDLE_STOP_MIN).as_str());
        
        let action = gaming_policy::decide(eff, apps_up, ollama_loaded, idle_long);
        
        if let Action::StopAll { unload } = action {
            // At most one stop per minute (the runner takes jobs on its next report).
            let recent = {
                let mut seen = SEEN.lock().unwrap();
                let entry = seen.entry(m.id.clone()).or_default();
                let recent = entry.sent_at.is_some_and(|t| t.elapsed() < Duration::from_secs(60));
                if !recent {
                    entry.sent_at = Some(Instant::now());
                }
                recent
            };
            if recent {
                continue;
            }

            if apps_up {
                let how = if eff == GpuMode::Gaming { "kill" } else { "stop" };
                send(s, m, json!({"tool": "gpu_apps", "action": how})).await;
                let _ = sqlx::query("UPDATE machines SET apps_stopped = 1 WHERE id = ?")
                    .bind(&m.id)
                    .execute(&s.db)
                    .await;
            }
            
            if unload {
                send(s, m, json!({"tool": "gpu_apps", "action": "unload_ollama"})).await;
            }
            
            hurry(&m.id);
            tracing::info!(machine = %m.name, mode = eff.as_str(), unload, "GPU-01: freeing the GPU");
            s.bus.send_all(Event::Changed { what: "gpus", machine_id: None });
        }
    }
}

/// GPU-02: whether studio jobs may go to this computer now (its GPU-01 mode is not off).
/// Computers without the "gpu" grant or without studio apps are not managed, so they may.
pub async fn studio_allowed(s: &AppState, machine: &str) -> bool {
    let Some(m) = machines(s).await.into_iter().find(|m| m.name == machine) else {
        return true;
    };
    if !m.granted {
        return true;
    }
    effective_of(s, &m).await.0 != GpuMode::Gaming
}

pub async fn ensure_started(s: &AppState, gpu: &str, app: &str) -> Result<(), String> {
    let Some(g) = s.config.gpus.iter().find(|g| g.id == gpu && g.apps.iter().any(|a| a == app)) else {
        return Ok(());
    };
    
    let Some(m) = machines(s).await.into_iter().find(|m| m.name == g.machine && m.granted) else {
        return Ok(());
    };
    
    let (eff, gaming) = effective_of(s, &m).await;
    
    if gaming && eff == GpuMode::Gaming {
        return Err(format!("{} is running a game (or ran one in the last 10 minutes), so Auto turned its studio off; switch it to Studio on to use it anyway.", m.name));
    }
    
    if eff == GpuMode::Gaming {
        return Err(format!("The studio is off on {}; switch it to Studio on or Auto first.", m.name));
    }
    
    let _ = sqlx::query("UPDATE machines SET studio_at = ?, apps_stopped = 0 WHERE id = ?")
        .bind(util::now())
        .bind(&m.id)
        .execute(&s.db)
        .await;
    
    // KS-02: one flag covers all apps of the machine, so always ask (docker start is a no-op when up).
    {
        send(s, &m, json!({"tool": "gpu_apps", "action": "start", "app": app})).await;
        tracing::info!(machine = %m.name, app, "GPU-01: starting on {}", m.name);
        s.bus.send_all(Event::Changed { what: "gpus", machine_id: None });
    }
    
    Ok(())
}

#[derive(Deserialize)]
pub struct ModeBody { 
    mode: String 
}

pub async fn list(State(s): State<AppState>) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(modes(&s).await))
}

/// Mode, effective mode and app state per computer with studio apps (also on the Capabilities page).
pub async fn modes(s: &AppState) -> Vec<Value> {
    let machines_list = machines(s).await;
    let mut result = Vec::new();
    
    for m in &machines_list {
        let (eff, gaming) = effective_of(s, m).await;
        
        let gpus_on_machine = s.config.gpus.iter()
            .filter(|g| g.machine == m.name && !g.apps.is_empty())
            .map(|g| g.id.clone())
            .collect::<Vec<_>>();
        
        let apps_flat = s.config.gpus.iter()
            .filter(|g| g.machine == m.name && !g.apps.is_empty())
            .flat_map(|g| g.apps.clone())
            .collect::<Vec<_>>();
        
        result.push(json!({
            "machine": m.name,
            "mode": m.mode,
            "effective": eff,
            // A game running now (the cooldown alone shows as Auto keeping the studio off).
            "gaming": gaming && s.host.gaming(&m.id).unwrap_or(false),
            "appsStopped": m.apps_stopped,
            "granted": m.granted,
            "studioAt": m.studio_at,
            "gpus": gpus_on_machine,
            "apps": apps_flat
        }));
    }
    
    result
}

pub async fn set(State(s): State<AppState>, Extension(u): Extension<User>, Path(machine): Path<String>, Json(b): Json<ModeBody>) -> ApiResult<Json<Value>> {
    if !crate::admin::is_admin(&s.db, &u.id).await? {
        return Err(ApiError::Forbidden("Only admins can change a computer's GPU mode.".into()));
    }
    
    let Some(mode) = GpuMode::parse(&b.mode) else {
        return Err(ApiError::BadRequest("Pick studio, gaming or auto.".into()));
    };
    
    let m = machines(&s).await.into_iter()
        .find(|m| m.name == machine)
        .ok_or_else(|| ApiError::NotFound)?;
    
    sqlx::query("UPDATE machines SET gpu_mode = ? WHERE id = ?")
        .bind(mode.as_str())
        .bind(&m.id)
        .execute(&s.db)
        .await?;
    
    tracing::info!(machine = %m.name, mode = mode.as_str(), by = %u.name, "GPU-01: mode set");
    if mode == GpuMode::Gaming {
        off_now(&s, &m).await;
    }
    hurry(&m.id);
    super::jobs::KICK.notify_one();
    s.bus.send_all(Event::Changed { what: "gpus", machine_id: None });
    
    Ok(Json(json!({"machine": m.name, "mode": mode})))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn state() -> AppState {
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        for q in [
            "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u', 'kees', '-', '2026-10-04')",
            "INSERT INTO machines (id, user_id, name, token_hash, created_at) VALUES ('m', 'u', 'souc', 'h', '2026-10-04')",
            "INSERT INTO machine_grants (machine_id, target, rights, granted_by, granted_at) VALUES ('m', 'system', '[\"gpu\"]', 'u', '2026-10-04')",
        ] {
            sqlx::query(q).execute(&db).await.unwrap();
        }
        let cfg = toml::from_str("[[gpu]]\nid = \"rx9070\"\nmachine = \"souc\"\npci = \"x\"\nvram_gb = 16\napps = [\"comfyui\"]").unwrap();
        AppState::for_tests(cfg, db)
    }

    /// GPU-04: the CLI gets no runner reports; the game cooldown stored by the server still
    /// keeps studio jobs off that computer in Auto (Studio on ignores it).
    #[tokio::test]
    async fn the_stored_game_cooldown_turns_the_studio_off() {
        let s = state().await;
        assert!(studio_allowed(&s, "souc").await);
        sqlx::query("UPDATE machines SET gaming_until = ?").bind(util::in_minutes(5)).execute(&s.db).await.unwrap();
        assert!(!studio_allowed(&s, "souc").await);
        assert!(ensure_started(&s, "rx9070", "comfyui").await.is_err());
        sqlx::query("UPDATE machines SET gpu_mode = 'studio'").execute(&s.db).await.unwrap();
        assert!(studio_allowed(&s, "souc").await);
        assert!(ensure_started(&s, "rx9070", "comfyui").await.is_ok());
        sqlx::query("UPDATE machines SET gpu_mode = 'auto', gaming_until = ?").bind(util::minutes_ago(1)).execute(&s.db).await.unwrap();
        assert!(studio_allowed(&s, "souc").await);
    }

    /// BUG-02: Studio off kills the apps, unloads Ollama and ends this computer's studio jobs at once.
    #[tokio::test]
    async fn studio_off_stops_everything_at_once() {
        let s = state().await;
        for (id, st, gpu, gpus, what) in [
            ("j1", "queued", None, "[\"rx9070\"]", "studio:moss-sfx:sfx"),
            ("j2", "running", Some("rx9070"), "[\"rx9070\"]", "studio:comfyui:oc-sheet"),
            ("j3", "queued", None, "[\"a770\"]", "studio:comfyui:oc-sheet"),
            ("j4", "running", Some("rx9070"), "[\"rx9070\"]", "chat"),
        ] {
            sqlx::query("INSERT INTO gpu_job (id, kind, what, gpus, vram_mib, ram_mib, state, gpu, created_at, beat_at) VALUES (?, 'asset', ?, ?, 1, 1, ?, ?, '2026', '2026')")
                .bind(id).bind(what).bind(gpus).bind(st).bind(gpu).execute(&s.db).await.unwrap();
        }
        let w = crate::studio::watchdog::watch("run-off", "rx9070");
        let m = machines(&s).await.into_iter().next().unwrap();
        off_now(&s, &m).await;
        for (id, want) in [("j1", "dropped"), ("j2", "dropped"), ("j3", "queued"), ("j4", "running")] {
            let (st, err): (String, Option<String>) = sqlx::query_as("SELECT state, error FROM gpu_job WHERE id = ?").bind(id).fetch_one(&s.db).await.unwrap();
            assert_eq!(st, want, "{id}");
            if want == "dropped" {
                assert_eq!(err.as_deref(), Some("Studio turned off"));
            }
        }
        let tools: Vec<String> = sqlx::query_scalar("SELECT tool FROM machine_jobs").fetch_all(&s.db).await.unwrap();
        assert!(tools.iter().any(|t| t.contains("\"kill\"")), "{tools:?}");
        assert!(tools.iter().any(|t| t.contains("unload_ollama")), "{tools:?}");
        let stopped: i64 = sqlx::query_scalar("SELECT apps_stopped FROM machines").fetch_one(&s.db).await.unwrap();
        assert_eq!(stopped, 1);
        let r = crate::studio::watchdog::guard(&w, Duration::from_secs(5), async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok::<_, anyhow::Error>(())
        })
        .await;
        assert!(r.unwrap_err().to_string().contains("turned off"));
    }

    /// GPU-04: managed computers report every 20 s at most, every 3 s right after a change.
    #[test]
    fn managed_computers_report_often_and_faster_after_a_change() {
        assert_eq!(report_interval("x1", 60), 60);
        MANAGED.lock().unwrap().insert("x1".into());
        assert_eq!(report_interval("x1", 60), 5);
        assert_eq!(report_interval("x1", 5), 5);
        hurry("x1");
        assert_eq!(report_interval("x1", 60), 3);
        assert_eq!(report_interval("x1", 1), 1);
        assert_eq!(report_interval("x2", 60), 60);
    }
}
