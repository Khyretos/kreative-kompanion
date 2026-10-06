//! STU-01: the team's Studio: make images from the image types (workflows with [studio]), list
//! your own runs, open their files. Any signed-in user; files only for their owner or an admin.
use axum::{Extension, Json, body::Body, extract::{Path, State}, http::{StatusCode, header}, response::Response};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use crate::{AppState, auth::User, error::{ApiError, ApiResult}};

/// Convert a size label to pixel dimensions.
pub fn size_px(size: &str) -> Option<(i64, i64)> {
    match size {
        "square" => Some((1024, 1024)),
        "wide" => Some((1344, 768)),
        "tall" => Some((768, 1344)),
        _ => None,
    }
}

/// Convert pixel dimensions back to a size label.
pub fn size_name(w: i64, h: i64) -> &'static str {
    // STU-02: by shape, so video sizes (1280 x 704) name the same way as images.
    if w > h { "wide" } else if h > w { "tall" } else { "square" }
}

/// Default count value for requests.
fn one() -> u32 { 1 }

#[derive(Deserialize)]
pub struct Make {
    #[serde(rename = "type")]
    kind: String,
    prompt: String,
    size: String,
    #[serde(default = "one")]
    count: u32,
}

/// POST /api/studio/make: generate images based on a workflow.
pub async fn make(State(s): State<AppState>, Extension(u): Extension<User>, Json(b): Json<Make>) -> ApiResult<(StatusCode, Json<Value>)> {
    let text = b.prompt.trim();
    if text.is_empty() {
        return Err(ApiError::BadRequest("Describe what to make.".into()));
    }
    if text.chars().count() > 500 {
        return Err(ApiError::BadRequest("Keep the description under 500 characters.".into()));
    }

    if b.count != 1 && b.count != 4 {
        return Err(ApiError::BadRequest("Make 1 or 4.".into()));
    }

    let workflows = super::workflow::load_all(&s.config.studio.workflows_dir);
    let studio = workflows
        .iter()
        .find_map(|(name, r)| r.as_ref().ok().filter(|_| *name == b.kind).and_then(|w| w.studio.clone()))
        .ok_or(ApiError::NotFound)?;
    let sizes = if studio.sizes.is_empty() {
        vec!["square".to_string()]
    } else {
        studio.sizes.clone()
    };

    if !sizes.contains(&b.size) {
        return Err(ApiError::BadRequest(format!("{} comes in {}", studio.label, sizes.join(", "))));
    }

    let (w_px, h_px) = studio
        .px
        .get(&b.size)
        .map(|p| (p[0], p[1]))
        .or_else(|| size_px(&b.size))
        .ok_or_else(|| ApiError::BadRequest("Pick square, wide or tall.".into()))?;

    let mut ids = Vec::new();
    for _ in 0..b.count {
        let mut params = Map::new();
        params.insert("prompt".into(), json!(text));
        params.insert("width".into(), json!(w_px));
        params.insert("height".into(), json!(h_px));
        params.insert("seed".into(), json!(-1));
        let (id, job) = super::queue(&s, &b.kind, "auto", &params, &u.id).await?;
        tokio::spawn(job);
        ids.push(id);
    }

    Ok((StatusCode::ACCEPTED, Json(json!({"ids": ids}))))
}

/// GET /api/studio/mine: list recent runs for the current user.
pub async fn mine(State(s): State<AppState>, Extension(u): Extension<User>) -> ApiResult<Json<Vec<Value>>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, String, String, Option<String>)>(
        "SELECT id, workflow, gpu, params, state, error, outputs, started_at, ended_at FROM studio_run WHERE user_id = ? ORDER BY started_at DESC LIMIT 200"
    )
    .bind(&u.id)
    .fetch_all(&s.db)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        let params: Value = serde_json::from_str(&row.3).unwrap_or(Value::Null);
        let prompt = params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
        let width = params.get("width").and_then(|v| v.as_i64()).unwrap_or(0);
        let height = params.get("height").and_then(|v| v.as_i64()).unwrap_or(0);
        let size = size_name(width, height);

        let outputs: Vec<String> = serde_json::from_str(&row.6).unwrap_or_default();
        let run_id = row.0.clone();
        let files: Vec<String> = (0..outputs.len()).map(|i| format!("/api/studio/runs/{run_id}/files/{i}")).collect();

        result.push(json!({
            "id": row.0,
            "type": row.1,
            "gpu": row.2,
            "prompt": prompt,
            "size": size,
            "state": row.4,
            "error": row.5,
            "files": files,
            "startedAt": row.7,
            "endedAt": row.8
        }));
    }

    Ok(Json(result))
}

/// GET /api/studio/runs/{id}/files/{n}: serve a generated file.
pub async fn file(State(s): State<AppState>, Extension(u): Extension<User>, Path((id, n)): Path<(String, usize)>) -> ApiResult<Response> {
    let (user_id, outputs) = sqlx::query_as::<_, (String, String)>(
        "SELECT user_id, outputs FROM studio_run WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| ApiError::NotFound)?;

    if user_id != u.id && !crate::admin::is_admin(&s.db, &u.id).await? {
        return Err(ApiError::NotFound);
    }

    let outputs: Vec<String> = serde_json::from_str(&outputs).unwrap_or_default();
    let path_str = outputs.get(n).ok_or_else(|| ApiError::NotFound)?.clone();

    let root = tokio::fs::canonicalize(&s.config.studio.output_dir).await.map_err(|_| ApiError::NotFound)?;
    let path = tokio::fs::canonicalize(&path_str).await.map_err(|_| ApiError::NotFound)?;

    if !path.starts_with(&root) {
        return Err(ApiError::NotFound);
    }

    let bytes = tokio::fs::read(&path).await.map_err(|_| ApiError::NotFound)?;
    let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
    let ct = match ext.as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    };

    let resp = Response::builder()
        .header(header::CONTENT_TYPE, ct)
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .body(Body::from(bytes))
        .map_err(|e| ApiError::from(anyhow::anyhow!(e)))?;

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_map_to_pixels() {
        assert_eq!(size_px("square"), Some((1024, 1024)));
        assert_eq!(size_px("wide"), Some((1344, 768)));
        assert_eq!(size_px("tall"), Some((768, 1344)));
        assert_eq!(size_px("huge"), None);
    }

    #[test]
    fn pixels_map_back_to_sizes() {
        assert_eq!(size_name(1344, 768), "wide");
        assert_eq!(size_name(768, 1344), "tall");
        assert_eq!(size_name(1024, 1024), "square");
        assert_eq!(size_name(0, 0), "square");
    }
}
