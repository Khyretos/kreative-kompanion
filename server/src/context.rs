//! CTX-01: how full a chat's context is (the meter in the composer), and a running summary of the
//! older part of a chat. Messages that no longer fit the prompt are summarised instead of silently
//! dropped: the model gets the summary, then the newest messages word for word.

use axum::{Extension, Json, extract::{Path, State}};
use futures::StreamExt;
use serde::Serialize;
use serde_json::json;

use crate::{AppState, api::RoleAssignment, auth::User, config::ProviderConfig, error::{ApiError, ApiResult}, events::Event, llm, util};

/// Characters of messages that go to the model word for word (older ones are summarised).
pub const HISTORY_CHARS: usize = 24_000;
/// Messages that go word for word at most.
pub const HISTORY_MESSAGES: usize = 40;
/// Share of the window (percent) at which the meter warns and the next answer summarises.
pub const WARN_PERCENT: i64 = 80;
/// Characters of older messages one summary reads at most (the newest of them when there are more).
const SOURCE_CHARS: usize = 60_000;
/// Characters of one message in the summary's input.
const SOURCE_MESSAGE: usize = 3_000;
/// Why the summary call is in the call log; it is not counted as the chat's context use.
pub const SUMMARY_REASON: &str = "Summarised the older messages of this chat.";

/// Tokens the model takes in at most: the provider's `context_window`, else a known size for common
/// cloud models (about; providers change them), else unknown.
pub fn window(p: Option<&ProviderConfig>, model: &str) -> Option<i64> {
    crate::context_windows::resolve(None, p, model).map(|r| r.0)
}

pub fn known_window(model: &str) -> Option<i64> {
    let m = model.to_ascii_lowercase();
    let w = if m.contains("[1m]") || m.ends_with("-1m") {
        1_000_000
    } else if m.contains("claude") {
        200_000
    } else if m.contains("gpt-4.1") {
        1_047_576
    } else if m.contains("gpt-5") {
        400_000
    } else if m.contains("gpt-4o") {
        128_000
    } else if m.starts_with("o3") || m.starts_with("o4") || m.contains("/o3") || m.contains("/o4") {
        200_000
    } else if m.contains("deepseek") {
        128_000
    } else if m.contains("grok-4") {
        256_000
    } else if m.contains("grok") {
        131_072
    } else if m.contains("gemini") {
        1_048_576
    } else if m.contains("codestral") || m.contains("devstral") {
        256_000
    } else if m.contains("mistral") || m.contains("magistral") {
        128_000
    } else {
        return None;
    };
    Some(w)
}

/// Tokens one call used of the context: what it read (cached parts included) plus what it wrote,
/// since the answer is part of the next prompt.
pub fn used(tokens_in: Option<i64>, tokens_out: Option<i64>, cache_write: Option<i64>, cache_read: Option<i64>) -> Option<i64> {
    tokens_in.map(|i| i + tokens_out.unwrap_or(0) + cache_write.unwrap_or(0) + cache_read.unwrap_or(0))
}

/// The meter's numbers for one chat.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub chat_id: String,
    pub used: Option<i64>,
    pub window: Option<i64>,
    pub model: Option<String>,
    pub summary: Option<SummaryInfo>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SummaryInfo {
    pub messages: i64,
    pub at: String,
    pub text: String,
}

/// The chat's latest call that reported its tokens (summary calls left out), with its model's window.
pub async fn usage(s: &AppState, chat_id: &str) -> sqlx::Result<Usage> {
    let row: Option<(String, String, Option<i64>, Option<i64>, Option<i64>, Option<i64>)> = sqlx::query_as(
        "SELECT provider_id, model_id, tokens_in, tokens_out, cache_write, cache_read FROM calls
         WHERE chat_id = ? AND tokens_in IS NOT NULL AND reason <> ? AND reason <> ? ORDER BY at DESC, rowid DESC LIMIT 1",
    )
    .bind(chat_id)
    .bind(SUMMARY_REASON)
    .bind(crate::attach_digest::DIGEST_REASON)
    .fetch_optional(&s.db)
    .await?;
    let summary = summary(&s.db, chat_id).await?.map(|x| SummaryInfo { messages: x.messages, at: x.at, text: x.text });
    Ok(match row {
        Some((provider, model, i, o, cw, cr)) => Usage {
            chat_id: chat_id.into(),
            used: used(i, o, cw, cr),
            window: crate::context_windows::window_for(s, &provider, &model),
            model: Some(model),
            summary,
        },
        None => Usage { chat_id: chat_id.into(), used: None, window: None, model: None, summary },
    })
}

/// Tells everyone who reads the chat its new numbers (the meter updates without a reload).
pub async fn announce(s: &AppState, chat_id: &str) {
    let (Ok(u), Ok(to)) = (usage(s, chat_id).await, crate::sharing::chat_audience(&s.db, chat_id).await) else { return };
    s.bus.send_many(&to, Event::Context { usage: u });
}

/// The running summary: covers every message up to (upto_at, upto_rowid).
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Summary {
    pub text: String,
    pub upto_at: String,
    pub upto_rowid: i64,
    pub messages: i64,
    pub at: String,
}

pub async fn summary(db: &sqlx::SqlitePool, chat_id: &str) -> sqlx::Result<Option<Summary>> {
    sqlx::query_as("SELECT text, upto_at, upto_rowid, messages, at FROM chat_summary WHERE chat_id = ?").bind(chat_id).fetch_optional(db).await
}

/// One stored message: (rowid, author, text, at).
pub type Row = (i64, String, String, String);

/// The messages the summary does not cover, oldest first (the newest `limit` of them).
pub async fn after_summary(db: &sqlx::SqlitePool, chat_id: &str, summary: Option<&Summary>, limit: i64) -> sqlx::Result<Vec<Row>> {
    let (at, rowid) = summary.map_or((String::new(), -1), |x| (x.upto_at.clone(), x.upto_rowid));
    let mut rows: Vec<Row> = sqlx::query_as(
        "SELECT rowid, author, text, at FROM messages WHERE chat_id = ?1 AND (at > ?2 OR (at = ?2 AND rowid > ?3))
         ORDER BY at DESC, rowid DESC LIMIT ?4",
    )
    .bind(chat_id)
    .bind(at)
    .bind(rowid)
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.reverse();
    Ok(rows)
}

/// How many of the newest `rows` fit in `budget` characters (at least one when there are any).
pub fn fit(rows: &[Row], budget: usize) -> usize {
    let mut left = budget;
    let n = rows
        .iter()
        .rev()
        .take_while(|r| {
            let fits = r.2.len() <= left;
            left = left.saturating_sub(r.2.len());
            fits
        })
        .count();
    n.max(usize::from(!rows.is_empty()))
}

/// What to do with the uncovered messages before an answer: how many of the newest go word for
/// word, and whether the ones before them are summarised first.
#[derive(Debug, PartialEq)]
pub struct Plan {
    pub keep: usize,
    pub summarise: bool,
}

/// Messages that fell out of the word-for-word part are summarised; the summary then also takes in
/// the older half of what still fits, so the next few answers need no new summary. Near a full
/// window (`near_full`), half of what is kept goes into the summary as well.
pub fn plan(rows: &[Row], near_full: bool) -> Plan {
    let fits = fit(rows, HISTORY_CHARS).min(HISTORY_MESSAGES);
    if fits >= rows.len() && !(near_full && rows.len() > 4) {
        return Plan { keep: rows.len(), summarise: false };
    }
    let mut keep = fit(rows, HISTORY_CHARS / 2).min(fits).min(HISTORY_MESSAGES / 2);
    if near_full {
        keep = keep.min(rows.len() / 2);
    }
    Plan { keep: keep.max(rows.len().min(2)), summarise: true }
}

fn clean(text: &str) -> String {
    let (body, files) = crate::chat_attach::split(text);
    let body = body.split("\n\n:::sources\n").next().unwrap_or("").trim().to_string();
    if files.is_empty() { body } else { format!("{body} [{} attached file(s)]", files.len()) }
}

/// The summary prompt's user part: the earlier summary, then the messages (cut to fit).
pub fn source(previous: Option<&str>, rows: &[Row]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut left = SOURCE_CHARS;
    for (_, author, text, at) in rows.iter().rev() {
        let mut t = clean(text);
        if t.chars().count() > SOURCE_MESSAGE {
            t = format!("{}… [cut]", t.chars().take(SOURCE_MESSAGE).collect::<String>());
        }
        if t.len() > left {
            break;
        }
        left -= t.len();
        let who = if author == "user" { "User" } else { "Assistant" };
        parts.push(format!("[{} {who}]\n{t}", at.get(..16).unwrap_or(at).replace('T', " ")));
    }
    parts.reverse();
    let skipped = rows.len() - parts.len();
    let mut out = String::new();
    if let Some(p) = previous {
        out.push_str(&format!("Summary of the chat before these messages:\n{p}\n\n"));
    }
    if skipped > 0 {
        out.push_str(&format!("({skipped} older messages are left out here: too long.)\n\n"));
    }
    out.push_str("Messages:\n\n");
    out.push_str(&parts.join("\n\n"));
    out
}

const SUMMARY_PROMPT: &str = "You keep the memory of a long chat between a user and their assistant. Write a summary of the \
chat so far that the assistant reads instead of the older messages: what the user wants and decided, facts, names, file paths, \
commands and numbers that matter, what was done and what is still open, and the user's preferences. Keep everything from the \
earlier summary that still matters. Write plain notes, newest state last, at most 400 words, in the chat's language. No preamble.";

/// Summarises `rows` (oldest first, all after the current summary) into a new summary covering up to
/// the last of them, with the chat's model at low effort; the call is logged like any other.
pub async fn compact(s: &AppState, user_id: &str, chat_id: &str, role: &RoleAssignment, rows: &[Row]) -> anyhow::Result<Summary> {
    let Some(last) = rows.last() else { anyhow::bail!("there is nothing to summarise") };
    let Some(p) = s.provider(&role.provider_id) else { anyhow::bail!("the provider {} is not configured", role.provider_id) };
    let previous = summary(&s.db, chat_id).await?;
    let (p, model) = crate::effort::apply(&p, &role.model_id, crate::effort::Effort::Low);
    let convo = vec![
        llm::ChatMessage::text("system", SUMMARY_PROMPT.into()),
        llm::ChatMessage::text("user", source(previous.as_ref().map(|x| x.text.as_str()), rows)),
    ];
    let started = std::time::Instant::now();
    let (mut text, mut usage, mut error) = (String::new(), json!({}), None::<String>);
    match llm::stream_chat(&s.http, &p, &model, &convo).await {
        Err(e) => error = Some(format!("{e:#}")),
        Ok(stream) => {
            tokio::pin!(stream);
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(llm::Chunk::Text(t)) => text.push_str(&t),
                    Ok(llm::Chunk::Usage { tokens_in, tokens_out, cache_write, cache_read }) => {
                        usage = json!({ "prompt_tokens": tokens_in, "completion_tokens": tokens_out,
                            "cache_creation_input_tokens": cache_write, "cache_read_input_tokens": cache_read });
                    }
                    Err(e) => {
                        error = Some(format!("{e:#}"));
                        break;
                    }
                }
            }
        }
    }
    let text = text.trim().to_string();
    if error.is_none() && text.is_empty() {
        error = Some("the model wrote an empty summary".into());
    }
    let request = json!({ "messages": convo });
    crate::api::log_call(s, user_id, chat_id, None, &role.role, role, SUMMARY_REASON, &request, &json!(text), &usage,
        started.elapsed().as_millis(), error.as_deref(), crate::effort::Effort::Low).await;
    if let Some(e) = error {
        anyhow::bail!(e);
    }
    let new = Summary {
        text,
        upto_at: last.3.clone(),
        upto_rowid: last.0,
        messages: previous.as_ref().map_or(0, |x| x.messages) + rows.len() as i64,
        at: util::now(),
    };
    sqlx::query(
        "INSERT INTO chat_summary (chat_id, text, upto_at, upto_rowid, messages, at) VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (chat_id) DO UPDATE SET text = excluded.text, upto_at = excluded.upto_at,
         upto_rowid = excluded.upto_rowid, messages = excluded.messages, at = excluded.at",
    )
    .bind(chat_id)
    .bind(&new.text)
    .bind(&new.upto_at)
    .bind(new.upto_rowid)
    .bind(new.messages)
    .bind(&new.at)
    .execute(&s.db)
    .await?;
    Ok(new)
}

/// The summary as the start of the prompt says it.
pub fn prompt_block(x: &Summary) -> String {
    format!(
        "\n\nThis chat is long: its {} oldest messages are summarised here, the newer ones follow word for word.\n\n{}",
        x.messages, x.text
    )
}

/// GET /api/chats/{id}/context: the meter's numbers and the summary, for anyone who reads the chat.
pub async fn get(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Usage>> {
    if !crate::api::can_read_chat(&s, &id, &u).await? {
        return Err(ApiError::NotFound);
    }
    Ok(Json(usage(&s, &id).await?))
}

/// POST /api/chats/{id}/compact: summarise now, all but the newest two messages (the owner or an editor).
pub async fn compact_now(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Usage>> {
    let Some(owner) = crate::sharing::chat_editor(&s.db, &u.id, &id).await? else {
        return Err(ApiError::NotFound);
    };
    let overseer = crate::overseer::chat_on(&s.db, &id).await;
    let wanted = if overseer { "overseer" } else { "orchestrator" };
    let Some(role) = crate::api::user_roles(&s, &owner).await?.into_iter().find(|r| r.role == wanted) else {
        return Err(ApiError::BadRequest("No model is set for this chat's role yet. Pick one under Models and roles.".into()));
    };
    let current = summary(&s.db, &id).await?;
    let rows = after_summary(&s.db, &id, current.as_ref(), 400).await?;
    if rows.len() <= 2 {
        return Err(ApiError::BadRequest("There is nothing new to summarise yet.".into()));
    }
    compact(&s, &owner, &id, &role, &rows[..rows.len() - 2]).await.map_err(|e| ApiError::BadRequest(format!("The summary failed: {e:#}")))?;
    announce(&s, &id).await;
    Ok(Json(usage(&s, &id).await?))
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
