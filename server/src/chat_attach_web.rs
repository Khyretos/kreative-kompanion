//! CHAT-08: files attached to a chat message. A picture is kept as it is; a PDF, text or code file is kept as its
//! extracted text. Both sit in the chat's folder (chat_files) and are named in the message by a `:::files` block.

use axum::{Extension, Json, body::Bytes, extract::{Path, Query, State}, http::StatusCode};
use base64::Engine;
use serde::Deserialize;
use crate::{AppState, auth::User, chat_attach::{Attached, kind_for, sees_pictures}, chat_files, error::{ApiError, ApiResult}, knowledge};

pub const MAX_FILES: usize = 8;
/// Characters of one file / of one message's files that go to the model.
const PER_FILE: usize = 20_000;
const PER_MESSAGE: usize = 30_000;

#[derive(Deserialize)]
pub struct Upload { name: String }

/// Text of a document: PDF and HTML through the knowledge extractor, anything else as UTF-8 (refusing binary data).
async fn text_of(name: &str, bytes: &[u8]) -> anyhow::Result<String> {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    if ext == "pdf" || ext == "html" || ext == "htm" {
        return knowledge::extract(name, bytes).await;
    }
    if bytes.iter().take(8192).any(|b| *b == 0) { anyhow::bail!("that is not a text file") }
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// POST /api/chats/{id}/attachments?name=: stores one file for the next message and says how it was kept.
pub async fn upload(State(s): State<AppState>, Extension(u): Extension<User>, Path(chat_id): Path<String>, Query(q): Query<Upload>, body: Bytes) -> ApiResult<(StatusCode, Json<Attached>)> {
    if !crate::api::owns(&s, "chats", &chat_id, &u).await? { return Err(ApiError::NotFound) }
    let name: String = q.name.trim().rsplit(['/', '\\']).next().unwrap_or("").chars().take(200).collect();
    if name.is_empty() || body.is_empty() { return Err(ApiError::BadRequest("The file needs a name and content.".into())) }
    let Some(kind) = kind_for(&name) else {
        return Err(ApiError::BadRequest(format!("{name}: only pictures (PNG, JPG, WebP), PDFs, text and code files can be attached.")));
    };
    let dir = chat_files::dir_for(&s.config.database);
    let bad = |e: anyhow::Error| ApiError::BadRequest(format!("{name}: {e}"));
    let file = if kind == "image" {
        let ext = match name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).as_deref() { Some("png") => "png", Some("webp") => "webp", _ => "jpg" };
        chat_files::save_bytes(&dir, &chat_id, ext, &body).await.map_err(bad)?
    } else {
        let text = text_of(&name, &body).await.map_err(bad)?;
        if text.trim().is_empty() { return Err(ApiError::BadRequest(format!("{name}: no text found in the file."))) }
        let kept: String = text.chars().take(chat_files::MAX_BYTES / 4).collect();
        chat_files::save_bytes(&dir, &chat_id, "txt", kept.as_bytes()).await.map_err(bad)?
    };
    Ok((StatusCode::CREATED, Json(Attached { name, file, kind: kind.into() })))
}

/// Whether a message may carry these files: few, of known kinds, each stored under this chat.
pub fn valid(dir: &std::path::Path, chat_id: &str, files: &[Attached]) -> bool {
    files.len() <= MAX_FILES && files.iter().all(|f| {
        (f.kind == "image" || f.kind == "text") && !f.name.is_empty() && f.name.len() <= 200 && chat_files::path_for(dir, chat_id, &f.file).is_some()
    })
}

/// What the model reads for the files of one message: the text of documents (cut to the budget), a line per picture
/// it cannot see, and the pictures themselves as data URLs when it can.
pub async fn for_model(dir: &std::path::Path, chat_id: &str, files: &[Attached], model: &str, with_pictures: bool) -> (String, Vec<String>) {
    let (mut note, mut images, mut left) = (String::new(), Vec::new(), PER_MESSAGE);
    let sees = with_pictures && sees_pictures(model);
    for f in files {
        let Some(path) = chat_files::path_for(dir, chat_id, &f.file) else { continue };
        let Ok(bytes) = tokio::fs::read(&path).await else { continue };
        if f.kind == "image" {
            if sees {
                let mime = chat_files::content_type(&f.file);
                images.push(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
                note.push_str(&format!("\n\n[Attached picture: {}]", f.name));
            } else {
                note.push_str(&format!("\n\n[Attached picture: {}. You cannot see pictures; say so if it matters.]", f.name));
            }
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        let take = text.chars().count().min(PER_FILE).min(left);
        let shown: String = text.chars().take(take).collect();
        left -= take;
        let cut = if take < text.chars().count() { format!("\n[cut here: {} more characters not shown]", text.chars().count() - take) } else { String::new() };
        note.push_str(&format!("\n\n[Attached file: {}]\n{shown}{cut}\n[End of {}]", f.name, f.name));
    }
    (note, images)
}

#[cfg(test)]
#[path = "chat_attach_web_tests.rs"]
mod tests;
