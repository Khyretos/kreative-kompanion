//! GPU-03: where the studio runs, one setting for Kompanion and Kreative Studio: "auto", a GPU
//! id, or "off". Stored in the settings table (key "studio_target").
use axum::{Extension, Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use crate::{AppState, auth::User, config::GpuConfig, error::{ApiError, ApiResult}, events::Event, gpus::role_policy::Target};

pub const KEY: &str = "studio_target";

/// Returns true if the target is valid: "auto", "off", or an ID of a GPU with a ComfyUI.
pub fn valid(gpus: &[GpuConfig], target: &str) -> bool {
    matches!(target, "auto" | "off") || super::comfy_target(gpus, target).is_some()
}

/// Choices for the UI dropdown.
pub fn choices(gpus: &[GpuConfig], coder_gpu: Option<&str>) -> Vec<Value> {
    let mut result = Vec::new();
    
    // Auto
    result.push(json!({
        "value": "auto",
        "label": "Automatic",
        "cost": "The studio computer while its studio is on, else the fallback GPU"
    }));
    
    // GPUs with ComfyUI
    for g in gpus {
        if super::comfy_target(gpus, &g.id).is_none() { continue; }
        
        let cost = if Some(g.id.as_str()) == coder_gpu {
            "Coder pauses while it runs".to_string()
        } else {
            String::new()
        };
        
        result.push(json!({
            "value": g.id,
            "label": format!("{} ({})", g.machine, g.id),
            "cost": cost
        }));
    }
    
    // Off
    result.push(json!({
        "value": "off",
        "label": "Off",
        "cost": "No studio jobs; the studio apps are stopped"
    }));
    
    result
}

/// Get the current target from settings, defaulting to "auto".
pub async fn get(s: &AppState) -> String {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT value FROM settings WHERE key = ?"
    )
    .bind(KEY)
    .fetch_optional(&s.db)
    .await;
    
    match row {
        Ok(Some(v)) => v,
        Ok(None) | Err(_) => "auto".to_string(),
    }
}

/// GET /api/studio/target
pub async fn read(State(s): State<AppState>) -> ApiResult<Json<Value>> {
    let target = get(&s).await;
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM gpu_job WHERE kind = 'asset' AND state IN ('queued', 'running')"
    )
    .fetch_one(&s.db)
    .await?;
    
    Ok(Json(json!({
        "target": target,
        "choices": choices(&s.config.gpus, std::env::var("GPU_ROLE_GPU").ok().as_deref()),
        "queued": n
    })))
}

#[derive(Deserialize)]
pub struct Body { 
    target: String 
}

/// PUT /api/studio/target
pub async fn set(State(s): State<AppState>, Extension(u): Extension<User>, Json(b): Json<Body>) -> ApiResult<Json<Value>> {
    if !crate::admin::is_admin(&s.db, &u.id).await? {
        return Err(ApiError::Forbidden("Only admins can choose where the studio runs.".into()));
    }
    
    apply(&s, &b.target, &u.name).await
}

/// Store a new target and act on it (Off stops the studio everywhere); `by` is logged.
async fn apply(s: &AppState, target: &str, by: &str) -> ApiResult<Json<Value>> {
    if !valid(&s.config.gpus, target) {
        return Err(ApiError::BadRequest("Pick auto, off or a GPU with a ComfyUI.".into()));
    }
    
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
    )
    .bind(KEY)
    .bind(target)
    .execute(&s.db)
    .await?;
    
    if target == "off" {
        for g in &s.config.gpus {
            if !g.apps.is_empty() {
                let _ = sqlx::query("UPDATE machines SET gpu_mode = 'gaming' WHERE name = ?")
                    .bind(&g.machine)
                    .execute(&s.db)
                    .await;
            }
        }
        
        let r = crate::gpus::role::current();
        if let Some(r) = r {
            if r.mode == "artist" && r.switching.is_none() {
                let busy = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM gpu_job WHERE state = 'running' AND gpu = ?"
                )
                .bind(r.gpu.clone().unwrap_or_default())
                .fetch_optional(&s.db)
                .await;
                
                if busy.map(|b| b.unwrap_or(1) == 0).unwrap_or(false) {
                    tokio::spawn(crate::gpus::role::switch(s.clone(), Target::Coder, by.to_string()));
                }
            }
        }
    } else if let Some(g) = s.config.gpus.iter().find(|g| g.id == target && !g.apps.is_empty()) {
        let _ = sqlx::query("UPDATE machines SET gpu_mode = 'auto' WHERE name = ? AND gpu_mode = 'gaming'")
            .bind(&g.machine)
            .execute(&s.db)
            .await;
    }
    
    tracing::info!(target = %target, by = %by, "GPU-03: studio target set");
    crate::gpus::jobs::KICK.notify_one();
    s.bus.send_all(Event::Changed { what: "gpus", machine_id: None });
    
    Ok(Json(json!({"target": target})))
}

pub const TOKEN_ENV: &str = "KOMPANION_STUDIO_TOKEN";

/// GPU-03: the request carries the studio's service token (env KOMPANION_STUDIO_TOKEN; unset: off).
pub fn service_ok(expected: Option<&str>, headers: &axum::http::HeaderMap) -> bool {
    if expected.is_none_or(|s| s.is_empty()) {
        return false;
    }
    let auth = headers.get(axum::http::header::AUTHORIZATION);
    let Some(auth_str) = auth.and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some(stripped) = auth_str.strip_prefix("Bearer ") else {
        return false;
    };
    let given = stripped.trim();
    crate::util::sha256_hex(given) == crate::util::sha256_hex(expected.unwrap())
}

async fn service_guard(s: &AppState, headers: &axum::http::HeaderMap) -> ApiResult<()> {
    s.throttle.check("studio-token")?;
    if service_ok(std::env::var(TOKEN_ENV).ok().as_deref(), headers) {
        Ok(())
    } else {
        s.throttle.fail("studio-token");
        Err(ApiError::Unauthorized)
    }
}

pub async fn service_read(State(s): State<AppState>, headers: axum::http::HeaderMap) -> ApiResult<Json<Value>> {
    service_guard(&s, &headers).await?;
    read(State(s)).await
}

#[derive(Deserialize)]
pub struct ServiceBody {
    target: String,
    by: String,
}

pub async fn service_set(State(s): State<AppState>, headers: axum::http::HeaderMap, Json(b): Json<ServiceBody>) -> ApiResult<Json<Value>> {
    service_guard(&s, &headers).await?;
    let by: String = format!("Kreative Studio ({})", b.by.chars().take(80).collect::<String>());
    apply(&s, &b.target, &by).await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn gpus() -> Vec<GpuConfig> {
        #[derive(serde::Deserialize)]
        struct G { gpu: Vec<GpuConfig> }
        toml::from_str::<G>(r#"
[[gpu]]
id = "a770"
machine = "kireserver"
pci = "x"
vram_gb = 16
[[gpu.holder]]
name = "ComfyUI"
probe = "comfyui:http://comfyui:8188/"
[[gpu]]
id = "a580"
machine = "kireserver"
pci = "y"
vram_gb = 8
[[gpu]]
id = "rx9070"
machine = "soucouyant"
pci = "*"
vram_gb = 16
apps = ["comfyui"]
[[gpu.holder]]
name = "ComfyUI (soucouyant)"
kind = "app"
probe = "comfyui:http://192.168.178.80:8188"
"#).unwrap().gpu
    }
    
    #[test]
    fn valid_targets() {
        let g = gpus();
        assert!(valid(&g, "auto"));
        assert!(valid(&g, "off"));
        assert!(valid(&g, "a770"));
        assert!(valid(&g, "rx9070"));
        assert!(!valid(&g, "a580"));
        assert!(!valid(&g, "kireserver"));
    }
    
    #[test]
    fn choices_name_the_cost() {
        let c = choices(&gpus(), Some("a770"));
        let values: Vec<&str> = c.iter().map(|v| v["value"].as_str().unwrap()).collect();
        assert_eq!(values, ["auto", "a770", "rx9070", "off"]);
        assert_eq!(c[1]["cost"], "Coder pauses while it runs");
        assert_eq!(c[2]["cost"], "");
        assert_eq!(c[2]["label"], "soucouyant (rx9070)");
    }

    #[test]
    fn service_token_must_match() {
        let mut h = axum::http::HeaderMap::new();
        assert!(!service_ok(Some("s3cret"), &h));
        h.insert("authorization", "Bearer s3cret".parse().unwrap());
        assert!(service_ok(Some("s3cret"), &h));
        assert!(!service_ok(Some("other"), &h));
        assert!(!service_ok(None, &h));
        assert!(!service_ok(Some(""), &h));
    }

    #[test]
    fn service_token_needs_bearer() {
        let mut h = axum::http::HeaderMap::new();
        h.insert("authorization", "s3cret".parse().unwrap());
        assert!(!service_ok(Some("s3cret"), &h));
    }

}
