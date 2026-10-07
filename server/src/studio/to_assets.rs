//! STU-02b: send a finished Studio result to Assets: an asset row in the "@studio" pack (absolute path, never copied into the read-only library), linked to one of the user's projects, with the run's provenance.

use axum::{Extension, Json, extract::{Path, State}};
use serde::Deserialize;
use serde_json::{Value, json};
use crate::{AppState, auth::User, error::{ApiError, ApiResult}, util};

#[derive(Deserialize)]
pub struct SendTo { pub project: String, #[serde(default)] pub n: usize }

pub fn provenance(workflow: &str, gpu: &str, params: &str, models: &str, ended_at: Option<&str>) -> Value {
    let p = serde_json::from_str::<Value>(params).unwrap_or(Value::Null);
    let m = serde_json::from_str::<Value>(models).unwrap_or_else(|_| json!([]));
    json!({
        "source": "kompanion-studio",
        "workflow": workflow,
        "gpu": gpu,
        "params": p,
        "models": m,
        "created": ended_at
    })
}

pub async fn send(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Json(b): Json<SendTo>) -> ApiResult<Json<Value>> {
    let row: Option<(String, String, String, String, String, String, String, Option<String>)> = sqlx::query_as("SELECT workflow, gpu, user_id, params, models, state, outputs, ended_at FROM studio_run WHERE id = ?")
        .bind(&id)
        .fetch_optional(&s.db)
        .await?;
    let Some((workflow, gpu, user_id, params, models, state, outputs, ended_at)) = row else { return Err(ApiError::NotFound) };
    if user_id != u.id { return Err(ApiError::NotFound); }
    if state != "done" { return Err(ApiError::BadRequest("Only a finished result can go to Assets.".into())); }

    if !crate::api::owns(&s, "projects", &b.project, &u).await? { return Err(ApiError::NotFound); }

    let outputs: Vec<String> = serde_json::from_str(&outputs).unwrap_or_default();
    let path_str = outputs.get(b.n).ok_or(ApiError::NotFound)?;
    let root = tokio::fs::canonicalize(&s.config.studio.output_dir).await.map_err(|_| ApiError::NotFound)?;
    let path = tokio::fs::canonicalize(path_str).await.map_err(|_| ApiError::NotFound)?;
    if !path.starts_with(&root) { return Err(ApiError::NotFound); }
    let meta = tokio::fs::metadata(&path).await.map_err(|_| ApiError::NotFound)?;
    let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64);
    let path_s = path.to_string_lossy().to_string();
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ext = crate::assets::classify::ext_of(&name);
    let (category, rule) = crate::assets::classify::classify_why(&path_s, &ext);

    let pack_id: i64 = sqlx::query_scalar("INSERT INTO asset_pack (key, name, kind) VALUES ('@studio', 'Kompanion Studio', 'loose') ON CONFLICT(key) DO UPDATE SET missing_since = NULL RETURNING id").fetch_one(&s.db).await?;
    let prov = provenance(&workflow, &gpu, &params, &models, ended_at.as_deref()).to_string();
    let asset_id: i64 = sqlx::query_scalar("INSERT INTO asset (pack_id, container, path, name, ext, size, mtime, category, rule, provenance) VALUES (?, '', ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(container, path) DO UPDATE SET provenance = excluded.provenance, missing_since = NULL RETURNING id")
        .bind(pack_id)
        .bind(&path_s)
        .bind(&name)
        .bind(&ext)
        .bind(meta.len() as i64)
        .bind(mtime)
        .bind(category)
        .bind(&rule)
        .bind(&prov)
        .fetch_one(&s.db)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO project_asset (project_id, asset_id, added_at) VALUES (?, ?, ?)")
        .bind(&b.project)
        .bind(asset_id)
        .bind(util::now())
        .execute(&s.db)
        .await?;
    Ok(Json(json!({"assetId": asset_id, "project": b.project, "name": name})))
}

#[cfg(test)]
#[path = "to_assets_tests.rs"]
mod tests;
