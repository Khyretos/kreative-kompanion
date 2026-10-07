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
    /// STU-01c: general, sensitive, questionable or explicit (types with a rating).
    #[serde(default)]
    rating: Option<String>,
    /// KS-01: Kreative Studio's own seed (none: random).
    #[serde(default)]
    seed: Option<i64>,
    /// KS-01: video frames (the workflow's "length" parameter).
    #[serde(default)]
    length: Option<i64>,
}

pub async fn read_make(req: axum::extract::Request, s: &AppState) -> ApiResult<(Make, Option<(Vec<u8>, &'static str)>, f64)> {
    let is_multipart = req
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("multipart/form-data"));

    if !is_multipart {
        let axum::Json(b) = <axum::Json<Make> as axum::extract::FromRequest<AppState>>::from_request(req, s)
            .await
            .map_err(|e| crate::error::ApiError::BadRequest(e.body_text()))?;
        return Ok((b, None, 0.85));
    }

    let mut form = <axum::extract::Multipart as axum::extract::FromRequest<AppState>>::from_request(req, s)
        .await
        .map_err(|e| crate::error::ApiError::BadRequest(e.body_text()))?;

    let (mut kind, mut prompt, mut size, mut count, mut rating, mut seed, mut length, mut face, mut weight) = (String::new(), String::new(), String::new(), 1u32, None::<String>, None::<i64>, None::<i64>, None::<(Vec<u8>, &'static str)>, 0.85f64);

    while let Some(field) = form.next_field().await.map_err(|e| crate::error::ApiError::BadRequest(e.body_text()))? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "face" => {
                let ext = match field.content_type().unwrap_or("") {
                    "image/jpeg" => "jpg",
                    "image/png" => "png",
                    "image/webp" => "webp",
                    _ => return Err(crate::error::ApiError::BadRequest("The photo must be a JPEG, PNG or WebP.".into())),
                };
                let bytes = field.bytes().await.map_err(|e| crate::error::ApiError::BadRequest(e.body_text()))?;
                if bytes.len() > 12 * 1024 * 1024 {
                    return Err(crate::error::ApiError::BadRequest("Keep the photo under 12 MB.".into()));
                }
                if !bytes.is_empty() {
                    face = Some((bytes.to_vec(), ext));
                }
            }
            _ => {
                let text = field.text().await.map_err(|e| crate::error::ApiError::BadRequest(e.body_text()))?;
                match name.as_str() {
                    "type" => kind = text,
                    "prompt" => prompt = text,
                    "size" => size = text,
                    "count" => count = text.parse().unwrap_or(1),
                    "rating" => {
                        if !text.is_empty() {
                            rating = Some(text);
                        }
                    }
                    "seed" => seed = text.trim().parse().ok(),
                    "length" => length = text.trim().parse().ok(),
                    "face_weight" => {
                        weight = text.parse().map_err(|_| crate::error::ApiError::BadRequest("The face weight must be a number.".into()))?;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok((Make { kind, prompt, size, count, rating, seed, length }, face, weight))
}

/// POST /api/studio/make: generate images based on a workflow.
pub async fn make(State(s): State<AppState>, Extension(u): Extension<User>, req: axum::extract::Request) -> ApiResult<(StatusCode, Json<Value>)> {
    let (b, face, weight) = read_make(req, &s).await?;
    let text = b.prompt.trim();
    if text.is_empty() {
        return Err(ApiError::BadRequest("Describe what to make.".into()));
    }
    // KS-01: Kreative Studio allows 2000 characters (its face tags come in front).
    if text.chars().count() > 2000 {
        return Err(ApiError::BadRequest("Keep the description under 2000 characters.".into()));
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

    // STU-01d: only types with a face graph take a photo.
    if face.is_some() && !workflows.iter().any(|(n, r)| *n == b.kind && r.as_ref().is_ok_and(|w| w.face.is_some())) {
        return Err(ApiError::BadRequest(format!("{} takes no face photo.", studio.label)));
    }
    if face.is_some() && !(0.0..=1.2).contains(&weight) {
        return Err(ApiError::BadRequest("The face weight must be between 0 and 1.2.".into()));
    }
    // KS-02: Coder describes the face photo; its tags go in front (what the prompt says wins).
    let mut face_tags = String::new();
    if let Some((bytes, _)) = &face {
        let ai = crate::assets::ai::Ai::from_env(s.http.clone());
        let tags = async {
            let png = super::facetags::to_png(bytes).await?;
            super::facetags::describe(&ai, &png).await
        };
        match tokio::time::timeout(std::time::Duration::from_secs(90), tags).await {
            Ok(Ok(tags)) => face_tags = super::facetags::merge_tags(&tags, text),
            Ok(Err(e)) => tracing::warn!(error = ?e, "face tags"),
            Err(_) => tracing::warn!("face tags: no answer in 90 s"),
        }
    }
    let prompt = if face_tags.is_empty() { text.to_string() } else { format!("{face_tags}, {text}") };
    let mut ids = Vec::new();
    for _ in 0..b.count {
        let mut params = Map::new();
        params.insert("prompt".into(), json!(prompt));
        params.insert("width".into(), json!(w_px));
        params.insert("height".into(), json!(h_px));
        params.insert("seed".into(), json!(b.seed.unwrap_or(-1)));
        if let Some(l) = b.length {
            params.insert("length".into(), json!(l));
        }
        if let Some(r) = &b.rating {
            params.insert("rating".into(), json!(r));
        }
        if let Some((bytes, ext)) = &face {
            let dir = s.config.studio.output_dir.join("users").join(&u.id).join("faces");
            tokio::fs::create_dir_all(&dir).await.map_err(anyhow::Error::from)?;
            let path = dir.join(format!("{}.{ext}", crate::util::new_id()));
            tokio::fs::write(&path, bytes).await.map_err(anyhow::Error::from)?;
            params.insert("face".into(), json!(path.display().to_string()));
            params.insert("face_weight".into(), json!(weight));
        }
        // STU-01d: a refused run removes its photo copy.
        let (id, job) = match super::queue(&s, &b.kind, "auto", &params, &u.id).await {
            Ok(x) => x,
            Err(e) => {
                if let Some(p) = params.get("face").and_then(|v| v.as_str()) {
                    let _ = tokio::fs::remove_file(p).await;
                }
                return Err(e);
            }
        };
        tokio::spawn(job);
        ids.push(id);
    }

    Ok((StatusCode::ACCEPTED, Json(json!({"ids": ids, "faceTags": face_tags}))))
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
            "seconds": params["seconds"],
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
        Some("mp4") => "video/mp4",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
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
