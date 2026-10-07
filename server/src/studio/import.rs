//! KS-03: Kreative Studio's finished jobs are copied into Kompanion's studio library (studio_run), so both apps show one library.

use std::path::Path;
use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use crate::{AppState, auth::User, error::{ApiError, ApiResult}};

#[derive(Deserialize)]
pub struct Import {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)] pub prompt: String,
    pub seed: Option<i64>,
    pub seconds: Option<f64>,
    pub lyrics: Option<String>,
    pub state: String,
    pub error: Option<String>,
    pub created: i64,
    #[serde(default)] pub files: Vec<String>,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

pub fn valid_type(t: &str) -> bool {
    !t.is_empty() && t.len() <= 40 && t.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn target_name(id: &str, n: usize, src: &Path) -> String {
    let ext = src.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).unwrap_or_else(|| "bin".to_string());
    format!("{id}-{}.{ext}", n + 1)
}

pub fn started_at(ms: i64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(ms as i128 * 1_000_000)
        .ok()
        .map(|dt| dt.format(&Rfc3339).unwrap_or_default())
        .unwrap_or_default()
}

pub fn source_ok(src: &Path, shared: &Path, output: &Path) -> bool {
    src.starts_with(shared) && !src.starts_with(output)
}

pub async fn import(State(s): State<AppState>, Extension(u): Extension<User>, Json(b): Json<Import>) -> ApiResult<(StatusCode, Json<Value>)> {
    if !valid_id(&b.id) || !valid_type(&b.kind) {
        return Err(ApiError::BadRequest("Bad id or type.".into()));
    }
    if b.state != "done" && b.state != "failed" {
        return Err(ApiError::BadRequest("state must be done or failed.".into()));
    }
    if b.prompt.chars().count() > 4000 || b.files.len() > 16 {
        return Err(ApiError::BadRequest("Too long.".into()));
    }

    let exists: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM studio_run WHERE id = ?")
        .bind(&b.id)
        .fetch_optional(&s.db)
        .await?;
    if exists.is_some() {
        return Ok((StatusCode::OK, Json(json!({"imported": false}))));
    }

    let output = tokio::fs::canonicalize(&s.config.studio.output_dir).await.map_err(|e| ApiError::Internal(anyhow::anyhow!("output_dir: {e}")))?;
    let shared = output.parent().map(Path::to_path_buf).unwrap_or_else(|| output.clone());

    let dir = output.join("users").join(&u.id).join(&b.kind);
    tokio::fs::create_dir_all(&dir).await.map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    let mut outputs: Vec<String> = Vec::new();
    for (n, f) in b.files.iter().enumerate() {
        let src = tokio::fs::canonicalize(f).await.map_err(|_| ApiError::BadRequest(format!("File not found: {f}")))?;
        if !source_ok(&src, &shared, &output) {
            return Err(ApiError::BadRequest(format!("File outside the shared folder: {f}")));
        }
        let to = dir.join(target_name(&b.id, n, &src));
        tokio::fs::copy(&src, &to).await.map_err(|e| ApiError::Internal(anyhow::anyhow!("copy {f}: {e}")))?;
        outputs.push(to.to_string_lossy().into_owned());
    }

    let params = json!({"prompt": b.prompt, "seed": b.seed, "seconds": b.seconds, "lyrics": b.lyrics, "source": "kreative-studio"});
    let at = started_at(b.created);

    sqlx::query("INSERT INTO studio_run (id, workflow, gpu, user_id, params, models, state, error, outputs, started_at, ended_at) VALUES (?, ?, 'kreative-studio', ?, ?, '[]', ?, ?, ?, ?, ?)")
        .bind(&b.id)
        .bind(&b.kind)
        .bind(&u.id)
        .bind(params.to_string())
        .bind(&b.state)
        .bind(&b.error)
        .bind(serde_json::to_string(&outputs).unwrap_or_default())
        .bind(&at)
        .bind(&at)
        .execute(&s.db)
        .await?;

    Ok((StatusCode::CREATED, Json(json!({"imported": true}))))
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
