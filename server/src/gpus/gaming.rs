//! GPU-01: Studio / Gaming / Auto per computer with studio apps; the runner's `gpu_apps` tool
//! stops and starts them. Decisions come from gaming_policy.
use std::{collections::HashMap, sync::{LazyLock, Mutex}, time::{Duration, Instant}};
use axum::{Extension, Json, extract::{Path, State}};
use serde::Deserialize;
use serde_json::{Value, json};
use super::{gaming_policy::{self, Action, GpuMode}, ledger::GpuLedger};
use crate::{AppState, auth::User, error::{ApiError, ApiResult}, events::Event, util};

#[derive(Default)]
struct Seen { 
    was_gaming: bool, 
    game_end: Option<Instant>, 
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
}

async fn machines(s: &AppState) -> Vec<Machine> {
    let mut result: Vec<Machine> = Vec::new();
    for g in &s.config.gpus {
        if g.apps.is_empty() || result.iter().any(|m| m.name == g.machine) { continue; }
        let row = sqlx::query_as::<_, (String, String, String, i64, Option<String>, bool)>(
            "SELECT id, user_id, gpu_mode, apps_stopped, studio_at,
                    EXISTS(SELECT 1 FROM machine_grants g WHERE g.machine_id = machines.id AND g.target = 'system'
                           AND g.rights LIKE '%\"gpu\"%' AND (g.expires IS NULL OR g.expires > ?))
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
                });
            }
            Ok(None) | Err(_) => {}
        }
    }
    result
}

fn effective_of(s: &AppState, m: &Machine) -> (GpuMode, bool) {
    let gaming = s.host.gaming(&m.id).unwrap_or(false);
    
    let mut map = SEEN.lock().unwrap();
    let e = map.entry(m.id.clone()).or_default();
    
    if e.was_gaming && !gaming {
        e.game_end = Some(Instant::now());
    }
    
    e.was_gaming = gaming;
    
    let eff = gaming_policy::effective(m.mode, gaming, e.game_end.map(|t| t.elapsed()));
    drop(map);
    
    (eff, gaming)
}

async fn send(s: &AppState, m: &Machine, tool: Value) {
    if let Err(e) = crate::access::queue_job(&s.db, &m.id, &m.user_id, &tool, None).await {
        tracing::warn!(machine = %m.name, "GPU-01: can't queue {tool}: {e}");
    }
}

pub async fn step(s: &AppState, ledgers: &[GpuLedger]) {
    for m in machines(s).await.iter().filter(|m| m.granted) {
        let (eff, _) = effective_of(s, m);
        
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
        let idle_long = !busy && m.studio_at.as_deref().is_none_or(|t| t < util::minutes_ago(gaming_policy::IDLE_STOP_MIN).as_str());
        
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
                send(s, m, json!({"tool": "gpu_apps", "action": "stop"})).await;
                let _ = sqlx::query("UPDATE machines SET apps_stopped = 1 WHERE id = ?")
                    .bind(&m.id)
                    .execute(&s.db)
                    .await;
            }
            
            if unload {
                send(s, m, json!({"tool": "gpu_apps", "action": "unload_ollama"})).await;
            }
            
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
    effective_of(s, &m).0 != GpuMode::Gaming
}

pub async fn ensure_started(s: &AppState, gpu: &str, app: &str) -> Result<(), String> {
    let Some(g) = s.config.gpus.iter().find(|g| g.id == gpu && g.apps.iter().any(|a| a == app)) else {
        return Ok(());
    };
    
    let Some(m) = machines(s).await.into_iter().find(|m| m.name == g.machine && m.granted) else {
        return Ok(());
    };
    
    let (eff, gaming) = effective_of(s, &m);
    
    if gaming {
        return Err(format!("{} is running a game; studio jobs wait until it ends, or switch it to Studio.", m.name));
    }
    
    if eff == GpuMode::Gaming {
        return Err(format!("{} is in gaming mode; switch it to Auto or Studio first.", m.name));
    }
    
    let _ = sqlx::query("UPDATE machines SET studio_at = ?, apps_stopped = 0 WHERE id = ?")
        .bind(util::now())
        .bind(&m.id)
        .execute(&s.db)
        .await;
    
    if m.apps_stopped {
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
        let (eff, gaming) = effective_of(s, m);
        
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
            "gaming": gaming,
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
    super::jobs::KICK.notify_one();
    s.bus.send_all(Event::Changed { what: "gpus", machine_id: None });
    
    Ok(Json(json!({"machine": m.name, "mode": mode})))
}
