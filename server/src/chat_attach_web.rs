//! CHAT-08: files attached to a chat message. A picture is kept as it is; a PDF, text or code file is kept as its
//! extracted text; any other file is kept as it is and only offered for download (the model is told its name and size).
//! All sit in the chat's folder (chat_files) and are named in the message by a `:::files` block.

use axum::{Extension, Json, body::Body, extract::{Path, Query, State}, http::StatusCode};
use base64::Engine;
use serde::Deserialize;
use crate::{AppState, auth::User, chat_attach::{Attached, SNIFF, kind_for, looks_like_text, sees_pictures}, chat_files, error::{ApiError, ApiResult}, knowledge};

pub const MAX_FILES: usize = 8;
/// A picture bigger than this is not sent to the model (providers refuse it); the model is told it exists.
const MAX_MODEL_PICTURE: u64 = 20 * 1024 * 1024;
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

fn size_label(bytes: u64) -> String {
    if bytes < 1024 { format!("{bytes} B") } else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) } else { format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0) }
}

/// A file received from the browser: where it was written, how big it is and its first bytes.
struct Received { path: std::path::PathBuf, size: u64, head: Vec<u8> }

/// The user's largest attachment in bytes (Settings; 0 = no limit, the default).
async fn upload_limit(s: &AppState, user_id: &str) -> Option<u64> {
    let mb: i64 = sqlx::query_scalar("SELECT max_upload_mb FROM users WHERE id = ?").bind(user_id).fetch_optional(&s.db).await.ok().flatten().unwrap_or(0);
    (mb > 0).then(|| mb as u64 * 1024 * 1024)
}

/// Streams the body to a temporary file of the chat (never all in memory, so any size works), stopping at `limit` bytes.
async fn receive(dir: &std::path::Path, chat_id: &str, body: Body, limit: Option<u64>) -> anyhow::Result<Received> {
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    let (_, path) = chat_files::new_file(dir, chat_id, "part").await?;
    let mut file = tokio::fs::File::create(&path).await?;
    let (mut size, mut head) = (0u64, Vec::new());
    let mut stream = body.into_data_stream();
    let done: anyhow::Result<()> = async {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            size += chunk.len() as u64;
            if let Some(max) = limit {
                if size > max { anyhow::bail!("larger than the {} MB set in Settings (Largest attachment)", max / 1024 / 1024) }
            }
            if head.len() < SNIFF { let n = (SNIFF - head.len()).min(chunk.len()); head.extend_from_slice(&chunk[..n]); }
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(())
    }.await;
    if let Err(e) = done {
        drop(file);
        tokio::fs::remove_file(&path).await.ok();
        return Err(e);
    }
    Ok(Received { path, size, head })
}

/// Keeps a received file by its kind; returns the stored name. A picture or other file is renamed, a document becomes its text.
async fn keep(dir: &std::path::Path, chat_id: &str, name: &str, kind: &str, got: &Received) -> anyhow::Result<String> {
    use tokio::io::AsyncReadExt;
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    if kind == "image" || kind == "file" {
        let ext = if kind == "file" { "bin" } else { match ext.as_str() { "png" => "png", "webp" => "webp", _ => "jpg" } };
        let (stored, to) = chat_files::new_file(dir, chat_id, ext).await?;
        tokio::fs::rename(&got.path, &to).await?;
        return Ok(stored);
    }
    let limit = chat_files::MAX_BYTES / 4;
    let text = if ext == "pdf" || ext == "html" || ext == "htm" {
        let text = text_of(name, &tokio::fs::read(&got.path).await?).await?;
        crate::util::cut_marked(&text, limit)
    } else {
        // Only the start of a big text file is kept; the cut says how much was left out.
        if got.head.iter().any(|b| *b == 0) { anyhow::bail!("that is not a text file") }
        let mut bytes = Vec::new();
        tokio::fs::File::open(&got.path).await?.take(limit as u64).read_to_end(&mut bytes).await?;
        let mut text = String::from_utf8_lossy(&bytes).into_owned();
        if got.size > bytes.len() as u64 {
            if text.ends_with('\u{FFFD}') { text.pop(); }
            text.push_str(&format!("\n[cut here: {} more bytes not kept]", got.size - bytes.len() as u64));
        }
        text
    };
    if text.trim().is_empty() { anyhow::bail!("no text found in the file.") }
    chat_files::save_bytes(dir, chat_id, "txt", text.as_bytes()).await
}

/// POST /api/chats/{id}/attachments?name=: stores one file for the next message and says how it was kept.
/// The body is streamed to disk; the size limit is the user's setting (none by default).
pub async fn upload(State(s): State<AppState>, Extension(u): Extension<User>, Path(chat_id): Path<String>, Query(q): Query<Upload>, body: Body) -> ApiResult<(StatusCode, Json<Attached>)> {
    if !crate::api::owns(&s, "chats", &chat_id, &u).await? { return Err(ApiError::NotFound) }
    let name: String = q.name.trim().rsplit(['/', '\\']).next().unwrap_or("").chars().take(200).collect();
    if name.is_empty() { return Err(ApiError::BadRequest("The file needs a name and content.".into())) }
    let dir = chat_files::dir_for(&s.config.database);
    let bad = |e: anyhow::Error| ApiError::BadRequest(format!("{name}: {e}"));
    let got = receive(&dir, &chat_id, body, upload_limit(&s, &u.id).await).await.map_err(bad)?;
    if got.size == 0 {
        tokio::fs::remove_file(&got.path).await.ok();
        return Err(ApiError::BadRequest("The file needs a name and content.".into()));
    }
    // An unknown extension (.lua, .tscn, .svg ...) that holds text is read like a text file; anything else is kept as it is.
    let kind = match kind_for(&name) { "file" if looks_like_text(&got.head, got.size > got.head.len() as u64) => "text", k => k };
    let kept = keep(&dir, &chat_id, &name, kind, &got).await;
    if kept.is_err() || kind == "text" { tokio::fs::remove_file(&got.path).await.ok(); }
    let file = kept.map_err(bad)?;
    Ok((StatusCode::CREATED, Json(Attached { name, file, kind: kind.into() })))
}

/// Whether a message may carry these files: few, of known kinds, each stored under this chat.
pub fn valid(dir: &std::path::Path, chat_id: &str, files: &[Attached]) -> bool {
    files.len() <= MAX_FILES && files.iter().all(|f| {
        ["image", "text", "file"].contains(&f.kind.as_str()) && !f.name.is_empty() && f.name.len() <= 200 && chat_files::path_for(dir, chat_id, &f.file).is_some()
    })
}

/// What the model reads for the files of one message: the text of documents (cut to the budget), a line per picture
/// it cannot see, and the pictures themselves as data URLs when it can.
pub async fn for_model(dir: &std::path::Path, chat_id: &str, files: &[Attached], model: &str, with_pictures: bool) -> (String, Vec<String>) {
    let (mut note, mut images, mut left) = (String::new(), Vec::new(), PER_MESSAGE);
    let sees = with_pictures && sees_pictures(model);
    for f in files {
        let Some(path) = chat_files::path_for(dir, chat_id, &f.file) else { continue };
        if f.kind == "file" {
            let Ok(meta) = tokio::fs::metadata(&path).await else { continue };
            note.push_str(&format!("\n\n[Attached file: {} ({}). It is not a text document, so its content is not shown to you; say so if it matters.]", f.name, size_label(meta.len())));
            continue;
        }
        if f.kind == "image" {
            let Ok(meta) = tokio::fs::metadata(&path).await else { continue };
            if meta.len() > MAX_MODEL_PICTURE {
                note.push_str(&format!("\n\n[Attached picture: {} ({}). It is too large to show you; say so if it matters.]", f.name, size_label(meta.len())));
                continue;
            }
        }
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
