//! CHAT-08: files attached to a chat message. A picture is kept as it is; a PDF, text or code file is kept as its
//! extracted text; any other file is kept as it is and only offered for download (the model is told its name and size).
//! All sit in the chat's folder (chat_files) and are named in the message by a `:::files` block.

use axum::{Extension, Json, body::Body, extract::{Path, Query, State}, http::StatusCode};
use base64::Engine;
use serde::Deserialize;
use crate::{AppState, attach_docs::{self, human}, attach_read::{self, Plan}, auth::User, chat_attach::{Attached, SNIFF, kind_for, looks_like_text, sees_pictures}, chat_files, error::{ApiError, ApiResult}, knowledge};

pub const MAX_FILES: usize = 8;
/// A picture bigger than this is not sent to the model (providers refuse it); the model is told it exists.
const MAX_MODEL_PICTURE: u64 = 20 * 1024 * 1024;
/// Below this many characters of room a file is only named, not shown.
const MIN_INLINE: usize = 400;

#[derive(Deserialize)]
pub struct Upload { name: String }

/// Text of a document: PDF and HTML through the knowledge extractor, anything else as UTF-8 (refusing binary data).
async fn text_of(name: &str, bytes: &[u8]) -> anyhow::Result<String> {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    if ext == "pdf" {
        return Ok(attach_docs::mark_pages(&knowledge::extract(name, bytes).await?));
    }
    if ext == "html" || ext == "htm" {
        return knowledge::extract(name, bytes).await;
    }
    if bytes.iter().take(8192).any(|b| *b == 0) { anyhow::bail!("that is not a text file") }
    Ok(String::from_utf8_lossy(bytes).into_owned())
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
            if let Some(max) = limit.filter(|max| size > *max) {
                anyhow::bail!("larger than the {} MB set in Settings (Largest attachment)", max / 1024 / 1024)
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

/// Keeps a received file by its kind; returns the stored name. A picture is renamed; a file of any other kind is renamed and
/// gets its text view next to it (same name, .txt); text is kept whole (a PDF or web page as its extracted text).
async fn keep(dir: &std::path::Path, chat_id: &str, name: &str, kind: &str, got: &Received) -> anyhow::Result<String> {
    let ext = attach_docs::ext_of(name);
    if kind == "image" {
        let ext = match ext.as_str() { "png" => "png", "webp" => "webp", _ => "jpg" };
        let (stored, to) = chat_files::new_file(dir, chat_id, ext).await?;
        tokio::fs::rename(&got.path, &to).await?;
        return Ok(stored);
    }
    if kind == "file" {
        let (stored, to) = chat_files::new_file(dir, chat_id, "bin").await?;
        tokio::fs::rename(&got.path, &to).await?;
        let view = attach_docs::view(name, &to, got.size, &got.head).await;
        tokio::fs::write(to.with_extension("txt"), view.text).await?;
        return Ok(stored);
    }
    if ext == "pdf" || ext == "html" || ext == "htm" {
        let text = text_of(name, &tokio::fs::read(&got.path).await?).await?;
        if text.trim().is_empty() { anyhow::bail!("no text found in the file.") }
        let (stored, to) = chat_files::new_file(dir, chat_id, "txt").await?;
        tokio::fs::write(to, text).await?;
        return Ok(stored);
    }
    // Text as it is: the model reads it from the file in parts, so none of it is cut.
    if got.head.contains(&0) { anyhow::bail!("that is not a text file") }
    if got.size as usize <= got.head.len() && got.head.iter().all(|b| b.is_ascii_whitespace()) { anyhow::bail!("no text found in the file.") }
    let (stored, to) = chat_files::new_file(dir, chat_id, "txt").await?;
    tokio::fs::rename(&got.path, &to).await?;
    Ok(stored)
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
    let kind = match kind_for(&name) { "file" if !attach_docs::has_reader(&name) && looks_like_text(&got.head, got.size > got.head.len() as u64) => "text", k => k };
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

/// What the model reads for the files of one message: each document's start (what fits the shared `plan`; the rest is
/// read with the attachment tools when the model has them), a line per file that has no text, a line per picture it
/// cannot see, and the pictures themselves as data URLs when it can.
pub async fn for_model(dir: &std::path::Path, chat_id: &str, files: &[Attached], model: &str, with_pictures: bool, plan: &mut Plan) -> (String, Vec<String>) {
    let (mut note, mut images) = (String::new(), Vec::new());
    let sees = with_pictures && sees_pictures(model);
    let mut docs_left = files.iter().filter(|f| f.kind != "image").count();
    for f in files {
        let d = attach_read::doc_of(dir, chat_id, f);
        let Some(original) = d.original.clone() else { continue };
        if f.kind == "image" {
            if d.size > MAX_MODEL_PICTURE {
                note.push_str(&format!("\n\n[Attached picture: {} ({}). It is too large to show you; say so if it matters.]", f.name, human(d.size)));
                continue;
            }
            let Ok(bytes) = tokio::fs::read(&original).await else { continue };
            if sees {
                let mime = chat_files::content_type(&f.file);
                images.push(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
                note.push_str(&format!("\n\n[Attached picture: {}]", f.name));
            } else {
                note.push_str(&format!("\n\n[Attached picture: {}. You cannot see pictures; say so if it matters.]", f.name));
            }
            continue;
        }
        let share = plan.left / docs_left.max(1);
        docs_left = docs_left.saturating_sub(1);
        let Some(text_path) = d.text.clone() else {
            note.push_str(&format!("\n\n[Attached file: {} ({}). It has no text to read; say so if it matters.]", f.name, human(d.size)));
            continue;
        };
        let page = if share >= MIN_INLINE {
            tokio::task::spawn_blocking(move || std::fs::File::open(&text_path).and_then(|t| attach_read::read_page(std::io::BufReader::new(t), 1, share))).await.ok().and_then(Result::ok)
        } else {
            None
        };
        let Some(page) = page else {
            plan.cut = true;
            note.push_str(&if plan.budget.tools {
                format!("\n\n[Attached file: {} ({}). Its text is not shown here: read it with {} name=\"{}\", or find words with {}.]", f.name, human(d.size), attach_read::READ, f.name, attach_read::SEARCH)
            } else {
                format!("\n\n[Attached file: {} ({}). Its text does not fit in what is shown to you.]", f.name, human(d.size))
            });
            continue;
        };
        plan.left = plan.left.saturating_sub(page.text.chars().count());
        plan.cut |= page.more;
        let end = if !page.more {
            format!("[End of {}]", f.name)
        } else if plan.budget.tools {
            format!("[Lines 1-{} of {} shown. The file goes on: read the rest with {} name=\"{}\" from_line={}, or find words with {}.]", page.last, f.name, attach_read::READ, f.name, page.last + 1, attach_read::SEARCH)
        } else {
            format!("[cut here: the rest of {} ({}) is not shown]", f.name, human(d.size))
        };
        note.push_str(&format!("\n\n[Attached file: {}]\n{}{end}", f.name, page.text));
    }
    (note, images)
}

#[cfg(test)]
#[path = "chat_attach_web_tests.rs"]
mod tests;
