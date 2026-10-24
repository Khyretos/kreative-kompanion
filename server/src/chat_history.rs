//! HIST-01: the model searches and reads the user's own past chats (`search_chats`, `read_chat`),
//! like the Claude app's chat search. Read-only, only the signed-in user's chats, results capped
//! (and saying so when cut), token-looking strings masked before they reach a model.

use serde_json::{Value, json};

/// Chats one search lists at most, and matching messages shown per chat.
const MAX_CHATS: usize = 10;
const SNIPPETS_PER_CHAT: usize = 3;
/// Messages one read_chat page holds at most (default 20).
const MAX_PAGE: i64 = 50;
const DEFAULT_PAGE: i64 = 20;
/// Characters one message may take in a read, and a whole read in all.
const MESSAGE_CAP: usize = 4_000;
/// The most a read returns when the caller allows more.
const READ_BUDGET: usize = 24_000;

pub const SEARCH: &str = "search_chats";
pub const READ: &str = "read_chat";

pub fn is_tool(name: &str) -> bool {
    name == SEARCH || name == READ
}

fn search_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Words to look for (every word must match, as a prefix)." },
            "limit": { "type": "integer", "description": "Chats to list, 1 to 10 (default 5)." }
        },
        "required": ["query"]
    })
}

fn read_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "chat_id": { "type": "string", "description": "The chat's id from search_chats (default: this chat)." },
            "before": { "type": "string", "description": "A message id: read the messages before it (for the next, older page)." },
            "limit": { "type": "integer", "description": "Messages to read, 1 to 50 (default 20)." }
        },
        "required": []
    })
}

const SEARCH_DESC: &str = "Search the user's past chats (titles and messages) for words. Returns the matching chats with their id, date and the matching lines. Use it when the user refers to something from another or an earlier chat, before saying you don't know. Answered by Kompanion itself, not a computer.";
const READ_DESC: &str = "Read messages of one of the user's chats, newest page first, oldest message first within the page. Without chat_id it reads this chat (also what is too old to be in your context). Answered by Kompanion itself, not a computer.";

/// The tools as an MCP server lists them (for chat tool rounds, chat_tools::run).
pub fn tools() -> Vec<Value> {
    vec![
        json!({ "name": SEARCH, "description": SEARCH_DESC, "inputSchema": search_params() }),
        json!({ "name": READ, "description": READ_DESC, "inputSchema": read_params() }),
    ]
}

/// The tools as function specs (for the PC agent's tool steps).
pub fn specs() -> Vec<Value> {
    vec![
        json!({ "type": "function", "function": { "name": SEARCH, "description": SEARCH_DESC, "parameters": search_params() } }),
        json!({ "type": "function", "function": { "name": READ, "description": READ_DESC, "parameters": read_params() } }),
    ]
}

/// Runs one of the two tools for `user_id`; `current` is the chat the model answers in, and a
/// read stays within `budget` characters of messages (the caller's cap on one tool result).
pub async fn call(db: &sqlx::SqlitePool, user_id: &str, current: &str, tool: &str, args: &Value, budget: usize) -> String {
    let limit = |d: i64, max: i64| args["limit"].as_i64().unwrap_or(d).clamp(1, max);
    let r = match tool {
        SEARCH => search(db, user_id, current, args["query"].as_str().unwrap_or(""), limit(5, MAX_CHATS as i64) as usize).await,
        READ => {
            let chat = args["chat_id"].as_str().map(str::trim).filter(|c| !c.is_empty()).unwrap_or(current);
            let before = args["before"].as_str().map(str::trim).filter(|b| !b.is_empty());
            read(db, user_id, current, chat, before, limit(DEFAULT_PAGE, MAX_PAGE), budget).await
        }
        _ => return format!("error: there is no tool {tool}"),
    };
    r.unwrap_or_else(|e| format!("error: {e}"))
}

fn day(at: &str) -> String {
    at.get(..16).unwrap_or(at).replace('T', " ")
}

fn one_line(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Matching chats, best first: id, title, project, last change and up to three matching lines.
pub async fn search(db: &sqlx::SqlitePool, user_id: &str, current: &str, query: &str, limit: usize) -> sqlx::Result<String> {
    let Some(fts) = crate::search::fts_query(query) else {
        return Ok("error: give words to search for".into());
    };
    // Message matches carry their chat in `parent`; a title match is the chat itself.
    let rows: Vec<(String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT kind, ref, parent, snippet(search_fts, 1, '[', ']', '…', 16)
         FROM search_fts WHERE search_fts MATCH ?1 AND user_id = ?2 AND kind IN ('chat', 'message')
         ORDER BY bm25(search_fts, 4.0, 1.0) LIMIT 200",
    )
    .bind(&fts)
    .bind(user_id)
    .fetch_all(db)
    .await?;
    let mut order: Vec<String> = Vec::new();
    let mut lines: std::collections::HashMap<String, Vec<(String, String)>> = Default::default();
    for (kind, id, parent, snippet) in rows {
        let chat = if kind == "chat" { id.clone() } else { parent.unwrap_or_default() };
        if chat.is_empty() {
            continue;
        }
        if !lines.contains_key(&chat) {
            if order.len() >= limit {
                continue;
            }
            order.push(chat.clone());
        }
        let l = lines.entry(chat).or_default();
        if kind == "message" && l.len() < SNIPPETS_PER_CHAT {
            l.push((id, snippet));
        }
    }
    if order.is_empty() {
        return Ok(format!("No chat matches \"{}\".", one_line(query)));
    }
    let mut out = format!("Chats matching \"{}\", best first:", one_line(query));
    for chat in &order {
        let meta: Option<(String, String, i64, Option<String>)> = sqlx::query_as(
            "SELECT c.title, c.updated_at, c.archived, p.name FROM chats c LEFT JOIN projects p ON p.id = c.project_id WHERE c.id = ? AND c.user_id = ?",
        )
        .bind(chat)
        .bind(user_id)
        .fetch_optional(db)
        .await?;
        let Some((title, updated, archived, project)) = meta else { continue };
        out.push_str(&format!("\n\n- chat_id {chat}: \"{}\", last message {}", one_line(&title), day(&updated)));
        if let Some(p) = project {
            out.push_str(&format!(", project \"{}\"", one_line(&p)));
        }
        if archived != 0 {
            out.push_str(", archived");
        }
        if chat == current {
            out.push_str(" (this chat)");
        }
        for (mid, snippet) in &lines[chat] {
            let who: Option<(String, String)> = sqlx::query_as("SELECT author, at FROM messages WHERE id = ?").bind(mid).fetch_optional(db).await?;
            let (author, at) = who.unwrap_or_default();
            let who = if author == "user" { "user" } else { "you" };
            out.push_str(&format!("\n  {} {who}: {}", day(&at), crate::secrets::mask(&one_line(snippet))));
        }
    }
    out.push_str("\n\nRead a chat with read_chat and its chat_id.");
    Ok(out)
}

/// One page of a chat's messages, oldest first, with the cursor for the page before it.
pub async fn read(db: &sqlx::SqlitePool, user_id: &str, current: &str, chat: &str, before: Option<&str>, limit: i64, budget: usize) -> sqlx::Result<String> {
    let meta: Option<(String,)> = sqlx::query_as("SELECT title FROM chats WHERE id = ? AND user_id = ?").bind(chat).bind(user_id).fetch_optional(db).await?;
    let Some((title,)) = meta else {
        return Ok(format!("error: there is no chat {chat} of yours"));
    };
    let cursor: Option<(String, i64)> = match before {
        Some(b) => {
            let c: Option<(String, i64)> = sqlx::query_as("SELECT at, rowid FROM messages WHERE id = ? AND chat_id = ?").bind(b).bind(chat).fetch_optional(db).await?;
            if c.is_none() {
                return Ok(format!("error: there is no message {b} in that chat"));
            }
            c
        }
        None => None,
    };
    let (at, rowid) = cursor.unwrap_or_else(|| ("\u{10ffff}".into(), i64::MAX));
    // One more than asked: tells whether older messages are left.
    let mut rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT id, author, text, at FROM messages WHERE chat_id = ?1 AND (at < ?2 OR (at = ?2 AND rowid < ?3))
         ORDER BY at DESC, rowid DESC LIMIT ?4",
    )
    .bind(chat)
    .bind(&at)
    .bind(rowid)
    .bind(limit + 1)
    .fetch_all(db)
    .await?;
    let mut more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    // Newest first until the budget is spent; what doesn't fit is left for the next page.
    let mut budget = budget.clamp(1_000, READ_BUDGET);
    let mut page: Vec<String> = Vec::new();
    let mut oldest: Option<String> = None;
    for (id, author, text, at) in &rows {
        let (body, files) = crate::chat_attach::split(text);
        let body = body.split("\n\n:::sources\n").next().unwrap_or("").trim().to_string();
        let mut body = crate::secrets::mask(&body);
        let total = body.chars().count();
        let cap = MESSAGE_CAP.min(budget.max(500));
        if total > cap {
            body = format!("{}… [cut: {} more characters]", body.chars().take(cap).collect::<String>(), total - cap);
        }
        if !files.is_empty() {
            body.push_str(&format!(" [{} attached file{}]", files.len(), if files.len() == 1 { "" } else { "s" }));
        }
        if !page.is_empty() && body.len() > budget {
            more = true;
            break;
        }
        budget = budget.saturating_sub(body.len());
        let who = if author == "user" { "user" } else { "you" };
        page.push(format!("[{} {who}, id {id}]\n{body}", day(at)));
        oldest = Some(id.clone());
    }
    if page.is_empty() {
        return Ok(format!("Chat \"{}\" has no {}messages.", one_line(&title), if before.is_some() { "older " } else { "" }));
    }
    page.reverse();
    let here = if chat == current { " (this chat)" } else { "" };
    let mut out = format!("Chat \"{}\"{here}, {} message{}, oldest first:\n\n{}", one_line(&title), page.len(), if page.len() == 1 { "" } else { "s" }, page.join("\n\n"));
    match (more, oldest) {
        (true, Some(o)) => out.push_str(&format!("\n\nOlder messages are left: read_chat with chat_id {chat} and before {o}.")),
        _ => out.push_str("\n\nThis is the start of the chat."),
    }
    Ok(out)
}

#[cfg(test)]
#[path = "chat_history_tests.rs"]
mod tests;
