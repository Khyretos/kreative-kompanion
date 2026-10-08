//! BLD-01: pictures from chat tools (e.g. Blender renders), stored per chat next to the database.
use std::path::{Path, PathBuf};
use axum::{Extension, body::Body, extract::{Path as UrlPath, State}, http::header, response::Response};
use base64::Engine;
use crate::{AppState, auth::User, error::{ApiError, ApiResult}};

pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// <database folder>/chat-files
pub fn dir_for(database: &Path) -> PathBuf {
    database.parent().unwrap_or(Path::new(".")).join("chat-files")
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn content_type(name: &str) -> &'static str {
    match name.rsplit_once('.') {
        Some((_, ext)) => match ext.to_lowercase().as_str() {
            "png" => "image/png",
            "jpg" => "image/jpeg",
            "webp" => "image/webp",
            "txt" | "md" => "text/plain; charset=utf-8",
            "json" => "application/json",
            "csv" => "text/csv",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            _ => "application/octet-stream",
        },
        None => "application/octet-stream",
    }
}

/// File types a chat tool may hand back besides pictures (CHAT-05): mime type to extension. They are only ever downloaded.
fn file_ext(mime: &str) -> Option<&'static str> {
    Some(match mime {
        "application/x-blender" | "application/x-blend" => "blend",
        "model/gltf-binary" => "glb",
        "application/zip" => "zip",
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        "text/markdown" => "md",
        "application/json" => "json",
        "text/csv" => "csv",
        _ => return None,
    })
}

const FILE_EXTS: [&str; 8] = ["blend", "glb", "zip", "pdf", "txt", "md", "json", "csv"];

/// Decodes and writes one picture or file; returns its file name (<uuid v4>.<ext>).
pub async fn save(dir: &Path, chat_id: &str, mime: &str, data_b64: &str) -> anyhow::Result<String> {
    if !safe_id(chat_id) { anyhow::bail!("bad chat id") }
    let ext = match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        other => file_ext(other).ok_or_else(|| anyhow::anyhow!("unsupported file type {other}"))?,
    };
    let bytes = base64::engine::general_purpose::STANDARD.decode(data_b64.trim())?;
    save_bytes(dir, chat_id, ext, &bytes).await
}

/// Writes bytes under the chat as <uuid v4>.<ext>; returns the file name.
pub async fn save_bytes(dir: &Path, chat_id: &str, ext: &str, bytes: &[u8]) -> anyhow::Result<String> {
    if !safe_id(chat_id) { anyhow::bail!("bad chat id") }
    if bytes.len() > MAX_BYTES { anyhow::bail!("picture too large") }
    let folder = dir.join(chat_id);
    tokio::fs::create_dir_all(&folder).await?;
    let name = format!("{}.{ext}", uuid::Uuid::new_v4());
    tokio::fs::write(folder.join(&name), bytes).await?;
    Ok(name)
}

/// The file of a picture when the ids are safe and it exists.
pub fn path_for(dir: &Path, chat_id: &str, name: &str) -> Option<PathBuf> {
    let (stem, ext) = name.rsplit_once('.')?;
    if !safe_id(chat_id) || uuid::Uuid::parse_str(stem).is_err() { return None }
    let ext = ext.to_lowercase();
    if !["png", "jpg", "webp"].contains(&ext.as_str()) && !FILE_EXTS.contains(&ext.as_str()) { return None }
    let p = dir.join(chat_id).join(name);
    p.is_file().then_some(p)
}

/// GET /api/chats/{id}/files/{name}: a picture of the user's own chat.
pub async fn file(State(s): State<AppState>, Extension(u): Extension<User>, UrlPath((id, name)): UrlPath<(String, String)>) -> ApiResult<Response> {
    if !crate::api::owns(&s, "chats", &id, &u).await? { return Err(ApiError::NotFound) }
    let path = path_for(&dir_for(&s.config.database), &id, &name).ok_or(ApiError::NotFound)?;
    let bytes = tokio::fs::read(&path).await.map_err(|_| ApiError::NotFound)?;
    let ctype = content_type(&name);
    let mut res = Response::builder()
        .header(header::CONTENT_TYPE, ctype)
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .header("X-Content-Type-Options", "nosniff");
    // CHAT-05: anything that is not a picture is saved, never opened in the page.
    if !ctype.starts_with("image/") {
        res = res.header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\""));
    }
    res
        .body(Body::from(bytes))
        .map_err(|e| ApiError::from(anyhow::anyhow!(e)))
}

#[cfg(test)]
#[path = "chat_files_tests.rs"]
mod tests;
