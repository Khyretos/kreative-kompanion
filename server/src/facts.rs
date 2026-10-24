//! FACTS: things worth remembering across chats, per project or per chat (docs/plans/facts.md).
//! The user adds, edits and deletes them in the Facts tab; the model reads them in its prompt and
//! saves its own with `remember` / `forget`. A value is free text (a note, a path, a URL, a place, a
//! command); `kind` only labels it. A value never holds a secret, only where one lives.

use axum::{Extension, Json, extract::{Path, Query, State}, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{AppState, auth::User, error::{ApiError, ApiResult}, events::Event, util};

pub const KINDS: &[&str] = &["text", "path", "url", "location", "other"];
const KEY_MAX: usize = 80;
const VALUE_MAX: usize = 2_000;
/// Characters the facts take in a prompt at most; past this the least recently used stay out.
const PROMPT_BUDGET: usize = 3_000;
pub const REMEMBER: &str = "remember";
pub const FORGET: &str = "forget";

#[derive(Debug, Clone, Serialize, sqlx::FromRow, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Fact {
    pub id: String,
    pub project_id: Option<String>,
    pub chat_id: Option<String>,
    pub key: String,
    pub value: String,
    pub kind: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_used_at: Option<String>,
    pub flagged_at: Option<String>,
}

const COLUMNS: &str = "id, project_id, chat_id, key, value, kind, source, created_at, updated_at, last_used_at, flagged_at";

/// Where a fact lives.
#[derive(Debug, Clone, PartialEq)]
pub enum Scope {
    Project(String),
    Chat(String),
}

impl Scope {
    fn of(f: &Fact) -> Scope {
        match (&f.project_id, &f.chat_id) {
            (Some(p), _) => Scope::Project(p.clone()),
            (_, Some(c)) => Scope::Chat(c.clone()),
            _ => Scope::Chat(String::new()),
        }
    }
}

/// A key: one line, 1 to 80 characters.
pub fn clean_key(k: &str) -> Result<String, String> {
    let k = k.split_whitespace().collect::<Vec<_>>().join(" ");
    if k.is_empty() || k.chars().count() > KEY_MAX {
        return Err(format!("A fact needs a name of 1 to {KEY_MAX} characters."));
    }
    Ok(k)
}

/// A value: kept exactly as written (trimmed), 1 to 2,000 characters, never a secret.
pub fn clean_value(v: &str) -> Result<String, String> {
    let v = v.trim();
    if v.is_empty() || v.chars().count() > VALUE_MAX {
        return Err(format!("A fact needs a value of 1 to {VALUE_MAX} characters."));
    }
    if crate::secrets::has_secret(v) {
        return Err("This looks like a password or token. Facts never hold secrets: write where it lives instead, for example \"in the password manager under Forgejo\" or \"file ~/.forgejo-token\".".into());
    }
    Ok(v.to_string())
}

pub fn clean_kind(k: Option<&str>) -> Result<String, String> {
    let k = k.map(str::trim).filter(|k| !k.is_empty()).unwrap_or("text");
    if KINDS.contains(&k) { Ok(k.to_string()) } else { Err(format!("The kind must be one of {}.", KINDS.join(", "))) }
}

pub async fn list(db: &sqlx::SqlitePool, scope: &Scope) -> sqlx::Result<Vec<Fact>> {
    let (col, id) = match scope {
        Scope::Project(p) => ("project_id", p),
        Scope::Chat(c) => ("chat_id", c),
    };
    // Flagged first, then by name.
    sqlx::query_as(&format!("SELECT {COLUMNS} FROM facts WHERE {col} = ? ORDER BY flagged_at IS NULL, lower(key)")).bind(id).fetch_all(db).await
}

async fn get_fact(db: &sqlx::SqlitePool, id: &str) -> sqlx::Result<Option<Fact>> {
    sqlx::query_as(&format!("SELECT {COLUMNS} FROM facts WHERE id = ?")).bind(id).fetch_optional(db).await
}

async fn by_key(db: &sqlx::SqlitePool, scope: &Scope, key: &str) -> sqlx::Result<Option<Fact>> {
    let (col, id) = match scope {
        Scope::Project(p) => ("project_id", p),
        Scope::Chat(c) => ("chat_id", c),
    };
    sqlx::query_as(&format!("SELECT {COLUMNS} FROM facts WHERE {col} = ? AND lower(key) = lower(?)")).bind(id).bind(key).fetch_optional(db).await
}

/// Who may read and who may change a scope: (read, write).
async fn rights(db: &sqlx::SqlitePool, user_id: &str, scope: &Scope) -> sqlx::Result<(bool, bool)> {
    match scope {
        Scope::Project(p) => Ok(match crate::sharing::access(db, user_id, p).await? {
            Some(a) => (true, a.role != crate::sharing::Role::Viewer),
            None => (false, false),
        }),
        Scope::Chat(c) => {
            let write = crate::sharing::chat_editor(db, user_id, c).await?.is_some();
            let read = write || crate::sharing::chat_audience(db, c).await?.iter().any(|u| u == user_id);
            Ok((read, write))
        }
    }
}

/// Everyone who sees the scope's facts: they reload the list.
async fn announce(s: &AppState, scope: &Scope) {
    let to: Vec<String> = match scope {
        Scope::Chat(c) => crate::sharing::chat_audience(&s.db, c).await.unwrap_or_default(),
        Scope::Project(p) => {
            let mut to: Vec<String> = sqlx::query_scalar("SELECT user_id FROM projects WHERE id = ? UNION SELECT user_id FROM project_members WHERE project_id = ?")
                .bind(p).bind(p).fetch_all(&s.db).await.unwrap_or_default();
            to.dedup();
            to
        }
    };
    s.bus.send_many(&to, Event::Changed { what: "facts", machine_id: None });
}

/// Adds a fact, or for the model changes its own fact with that key. A fact the user set is never
/// changed by the model. Returns the stored fact.
pub async fn put(db: &sqlx::SqlitePool, scope: &Scope, key: &str, value: &str, kind: Option<&str>, source: &str) -> Result<Fact, String> {
    let (key, value, kind) = (clean_key(key)?, clean_value(value)?, clean_kind(kind)?);
    let now = util::now();
    let existing = by_key(db, scope, &key).await.map_err(|e| e.to_string())?;
    let id = match existing {
        Some(f) if source == "user" => return Err(format!("There is already a fact named \"{}\": edit that one instead.", f.key)),
        Some(f) if f.source == "user" => return Err(format!("The user set \"{}\"; only they can change it (in Facts). Tell them what should change.", f.key)),
        Some(f) => {
            sqlx::query("UPDATE facts SET key = ?, value = ?, kind = ?, updated_at = ?, flagged_at = NULL WHERE id = ?")
                .bind(&key).bind(&value).bind(&kind).bind(&now).bind(&f.id).execute(db).await.map_err(|e| e.to_string())?;
            f.id
        }
        None => {
            let id = util::new_id();
            let (p, c) = match scope {
                Scope::Project(p) => (Some(p.as_str()), None),
                Scope::Chat(c) => (None, Some(c.as_str())),
            };
            sqlx::query("INSERT INTO facts (id, project_id, chat_id, key, value, kind, source, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(&id).bind(p).bind(c).bind(&key).bind(&value).bind(&kind).bind(source).bind(&now).bind(&now)
                .execute(db).await.map_err(|e| e.to_string())?;
            id
        }
    };
    get_fact(db, &id).await.map_err(|e| e.to_string())?.ok_or_else(|| "the fact is gone".to_string())
}

// ---- REST: the Facts tab ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    chat_id: Option<String>,
    project_id: Option<String>,
}

/// GET /api/facts?chatId= (the chat's project facts and its own) or ?projectId= (the project's).
pub async fn get(State(s): State<AppState>, Extension(u): Extension<User>, Query(q): Query<ListQuery>) -> ApiResult<Json<Value>> {
    let (project, chat) = match (q.chat_id.filter(|c| !c.is_empty()), q.project_id.filter(|p| !p.is_empty())) {
        (Some(c), _) => {
            let p: Option<Option<String>> = sqlx::query_scalar("SELECT project_id FROM chats WHERE id = ?").bind(&c).fetch_optional(&s.db).await?;
            let Some(p) = p else { return Err(ApiError::NotFound) };
            (p, Some(c))
        }
        (None, Some(p)) => (Some(p), None),
        (None, None) => return Err(ApiError::BadRequest("Name a chat or a project.".into())),
    };
    let mut out = json!({ "projectId": project, "chatId": chat });
    if let Some(c) = &chat {
        let (read, write) = rights(&s.db, &u.id, &Scope::Chat(c.clone())).await?;
        if !read {
            return Err(ApiError::NotFound);
        }
        out["chat"] = json!(list(&s.db, &Scope::Chat(c.clone())).await?);
        out["canEditChat"] = json!(write);
    }
    if let Some(p) = &project {
        let (read, write) = rights(&s.db, &u.id, &Scope::Project(p.clone())).await?;
        if !read && chat.is_none() {
            return Err(ApiError::NotFound);
        }
        out["project"] = json!(if read { list(&s.db, &Scope::Project(p.clone())).await? } else { Vec::new() });
        out["canEditProject"] = json!(write);
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewFact {
    scope: String,
    project_id: Option<String>,
    chat_id: Option<String>,
    key: String,
    value: String,
    kind: Option<String>,
}

fn scope_from(scope: &str, project: Option<String>, chat: Option<String>) -> ApiResult<Scope> {
    match (scope, project.filter(|p| !p.is_empty()), chat.filter(|c| !c.is_empty())) {
        ("project", Some(p), _) => Ok(Scope::Project(p)),
        ("chat", _, Some(c)) => Ok(Scope::Chat(c)),
        _ => Err(ApiError::BadRequest("A fact belongs to a project (with its id) or a chat (with its id).".into())),
    }
}

/// POST /api/facts: the user adds a fact (owner or editor of its project or chat).
pub async fn create(State(s): State<AppState>, Extension(u): Extension<User>, Json(b): Json<NewFact>) -> ApiResult<(StatusCode, Json<Fact>)> {
    let scope = scope_from(&b.scope, b.project_id, b.chat_id)?;
    match rights(&s.db, &u.id, &scope).await? {
        (_, true) => {}
        (true, false) => return Err(ApiError::Forbidden("You can read this project's facts but not change them.".into())),
        _ => return Err(ApiError::NotFound),
    }
    let f = put(&s.db, &scope, &b.key, &b.value, b.kind.as_deref(), "user").await.map_err(ApiError::BadRequest)?;
    announce(&s, &scope).await;
    Ok((StatusCode::CREATED, Json(f)))
}

#[derive(Deserialize)]
pub struct FactChange {
    key: Option<String>,
    value: Option<String>,
    kind: Option<String>,
}

async fn writable(s: &AppState, u: &User, id: &str) -> ApiResult<Fact> {
    let Some(f) = get_fact(&s.db, id).await? else { return Err(ApiError::NotFound) };
    match rights(&s.db, &u.id, &Scope::of(&f)).await? {
        (_, true) => Ok(f),
        (true, false) => Err(ApiError::Forbidden("You can read this project's facts but not change them.".into())),
        _ => Err(ApiError::NotFound),
    }
}

/// PATCH /api/facts/{id}: the user edits any fact; it becomes theirs (the model no longer changes
/// it) and its flag is cleared.
pub async fn update(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Json(b): Json<FactChange>) -> ApiResult<Json<Fact>> {
    let f = writable(&s, &u, &id).await?;
    let key = match b.key.as_deref() { Some(k) => clean_key(k).map_err(ApiError::BadRequest)?, None => f.key.clone() };
    let value = match b.value.as_deref() { Some(v) => clean_value(v).map_err(ApiError::BadRequest)?, None => f.value.clone() };
    let kind = match b.kind.as_deref() { Some(k) => clean_kind(Some(k)).map_err(ApiError::BadRequest)?, None => f.kind.clone() };
    let scope = Scope::of(&f);
    if let Some(other) = by_key(&s.db, &scope, &key).await?.filter(|o| o.id != f.id) {
        return Err(ApiError::BadRequest(format!("There is already a fact named \"{}\".", other.key)));
    }
    sqlx::query("UPDATE facts SET key = ?, value = ?, kind = ?, source = 'user', updated_at = ?, flagged_at = NULL WHERE id = ?")
        .bind(&key).bind(&value).bind(&kind).bind(util::now()).bind(&id).execute(&s.db).await?;
    announce(&s, &scope).await;
    Ok(Json(get_fact(&s.db, &id).await?.ok_or(ApiError::NotFound)?))
}

/// DELETE /api/facts/{id}.
pub async fn delete(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    let f = writable(&s, &u, &id).await?;
    sqlx::query("DELETE FROM facts WHERE id = ?").bind(&id).execute(&s.db).await?;
    announce(&s, &Scope::of(&f)).await;
    Ok(StatusCode::NO_CONTENT)
}

// ---- The model's side: the prompt block and remember / forget ----

fn params_remember() -> Value {
    json!({
        "type": "object",
        "properties": {
            "scope": { "type": "string", "enum": ["project", "chat"], "description": "project: for every chat of this chat's project; chat: only this chat." },
            "key": { "type": "string", "description": "A short name, e.g. \"deploy command\" or \"Forgejo token location\"." },
            "value": { "type": "string", "description": "The fact, exactly. Never a password or token: say where it lives instead." },
            "kind": { "type": "string", "enum": KINDS, "description": "Label only (default text)." }
        },
        "required": ["scope", "key", "value"]
    })
}

fn params_forget() -> Value {
    json!({
        "type": "object",
        "properties": {
            "scope": { "type": "string", "enum": ["project", "chat"] },
            "key": { "type": "string" }
        },
        "required": ["scope", "key"]
    })
}

const REMEMBER_DESC: &str = "Save a fact for later chats: something the user told you or you found that will matter again (a path, a command, a decision, a preference). Saving the same key again replaces your earlier value. Answered by Kompanion itself, not a computer.";
const FORGET_DESC: &str = "Delete a fact you saved that is wrong or no longer true. Answered by Kompanion itself, not a computer.";

pub fn is_tool(name: &str) -> bool {
    name == REMEMBER || name == FORGET
}

/// The tools as an MCP server lists them (chat tool rounds).
pub fn tools() -> Vec<Value> {
    vec![
        json!({ "name": REMEMBER, "description": REMEMBER_DESC, "inputSchema": params_remember() }),
        json!({ "name": FORGET, "description": FORGET_DESC, "inputSchema": params_forget() }),
    ]
}

/// The tools as function specs (PC agent steps).
pub fn specs() -> Vec<Value> {
    vec![
        json!({ "type": "function", "function": { "name": REMEMBER, "description": REMEMBER_DESC, "parameters": params_remember() } }),
        json!({ "type": "function", "function": { "name": FORGET, "description": FORGET_DESC, "parameters": params_forget() } }),
    ]
}

async fn model_scope(db: &sqlx::SqlitePool, chat_id: &str, scope: &str) -> Result<Scope, String> {
    match scope {
        "chat" => Ok(Scope::Chat(chat_id.to_string())),
        "project" => {
            let p: Option<Option<String>> = sqlx::query_scalar("SELECT project_id FROM chats WHERE id = ?").bind(chat_id).fetch_optional(db).await.map_err(|e| e.to_string())?;
            p.flatten().map(Scope::Project).ok_or_else(|| "This chat is not in a project: use scope \"chat\".".to_string())
        }
        _ => Err("scope must be \"project\" or \"chat\"".into()),
    }
}

/// Runs remember / forget for the model answering in `chat_id` (the chat's owner's tools).
pub async fn call(s: &AppState, chat_id: &str, tool: &str, args: &Value) -> String {
    let scope = match model_scope(&s.db, chat_id, args["scope"].as_str().unwrap_or("")).await {
        Ok(x) => x,
        Err(e) => return format!("error: {e}"),
    };
    let key = args["key"].as_str().unwrap_or("");
    let r = match tool {
        REMEMBER => put(&s.db, &scope, key, args["value"].as_str().unwrap_or(""), args["kind"].as_str(), "assistant")
            .await
            .map(|f| format!("Saved {} fact \"{}\".", if f.project_id.is_some() { "project" } else { "chat" }, f.key)),
        FORGET => match by_key(&s.db, &scope, key).await {
            Ok(Some(f)) if f.source == "user" => Err(format!("The user set \"{}\"; only they can delete it (in Facts).", f.key)),
            Ok(Some(f)) => sqlx::query("DELETE FROM facts WHERE id = ?").bind(&f.id).execute(&s.db).await.map(|_| format!("Deleted \"{}\".", f.key)).map_err(|e| e.to_string()),
            Ok(None) => Err(format!("There is no fact \"{key}\" there.")),
            Err(e) => Err(e.to_string()),
        },
        _ => Err(format!("there is no tool {tool}")),
    };
    match r {
        Ok(msg) => {
            announce(s, &scope).await;
            msg
        }
        Err(e) => format!("error: {e}"),
    }
}

/// "3 d ago" from an RFC 3339 time.
fn age(at: &str, now: time::OffsetDateTime) -> String {
    let Ok(t) = time::OffsetDateTime::parse(at, &time::format_description::well_known::Rfc3339) else { return String::new() };
    let m = (now - t).whole_minutes().max(0);
    match m {
        0..=59 => format!("{m} min ago"),
        60..=1439 => format!("{} h ago", m / 60),
        _ => format!("{} d ago", m / 1440),
    }
}

fn line(f: &Fact, now: time::OffsetDateTime) -> String {
    let who = if f.source == "user" { "set by the user" } else { "saved by you" };
    let flag = if f.flagged_at.is_some() { ", FLAGGED: may be out of date" } else { "" };
    let value = f.value.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("- {}: {value} ({}, {who}, {}{flag})", f.key, f.kind, age(&f.updated_at, now))
}

/// The facts block for a chat's prompt (its project's facts, then its own) and the ids in it. Over
/// the budget the least recently used stay out (they are kept). Marks the ones used now.
pub async fn prompt_block(db: &sqlx::SqlitePool, chat_id: &str, now: time::OffsetDateTime) -> Option<(String, Vec<String>)> {
    let project: Option<String> = sqlx::query_scalar("SELECT project_id FROM chats WHERE id = ?").bind(chat_id).fetch_optional(db).await.ok().flatten().flatten();
    let mut all: Vec<Fact> = match &project {
        Some(p) => list(db, &Scope::Project(p.clone())).await.ok()?,
        None => Vec::new(),
    };
    all.extend(list(db, &Scope::Chat(chat_id.to_string())).await.ok()?);
    if all.is_empty() {
        return None;
    }
    // Most recently used or changed first; that is the order they get room in.
    let mut by_use: Vec<&Fact> = all.iter().collect();
    by_use.sort_by(|a, b| {
        let ua = a.last_used_at.as_deref().unwrap_or("").max(a.updated_at.as_str());
        let ub = b.last_used_at.as_deref().unwrap_or("").max(b.updated_at.as_str());
        ub.cmp(ua)
    });
    let mut left = PROMPT_BUDGET;
    let mut ids: Vec<String> = Vec::new();
    for f in by_use {
        let n = line(f, now).len() + 1;
        if n > left {
            continue;
        }
        left -= n;
        ids.push(f.id.clone());
    }
    let shown = |chat: bool| -> Vec<String> {
        all.iter().filter(|f| f.chat_id.is_some() == chat && ids.contains(&f.id)).map(|f| line(f, now)).collect()
    };
    let (p, c) = (shown(false), shown(true));
    let mut out = String::from("\n\nFacts saved for this work (use them; save new ones with remember, delete wrong ones you saved with forget):");
    if !p.is_empty() {
        out.push_str(&format!("\nProject:\n{}", p.join("\n")));
    }
    if !c.is_empty() {
        out.push_str(&format!("\nThis chat:\n{}", c.join("\n")));
    }
    let left_out = all.len() - ids.len();
    if left_out > 0 {
        out.push_str(&format!("\n({left_out} older facts are not shown here.)"));
    }
    let stamp = util::now();
    for id in &ids {
        let _ = sqlx::query("UPDATE facts SET last_used_at = ? WHERE id = ?").bind(&stamp).bind(id).execute(db).await;
    }
    Some((out, ids))
}

/// A path fact whose path a step found missing gets flagged (shown as possibly out of date; never deleted).
pub async fn flag_missing_path(s: &AppState, chat_id: &str, path: &str) {
    let path = path.trim_end_matches('/');
    let rows: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT f.id, f.project_id, f.chat_id FROM facts f JOIN chats c ON c.id = ?1
         WHERE f.kind = 'path' AND f.flagged_at IS NULL AND (f.chat_id = c.id OR f.project_id = c.project_id)
         AND rtrim(trim(f.value), '/') = ?2",
    )
    .bind(chat_id)
    .bind(path)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    for (id, p, c) in rows {
        let _ = sqlx::query("UPDATE facts SET flagged_at = ? WHERE id = ?").bind(util::now()).bind(&id).execute(&s.db).await;
        let scope = match (p, c) {
            (Some(p), _) => Scope::Project(p),
            (_, Some(c)) => Scope::Chat(c),
            _ => continue,
        };
        announce(s, &scope).await;
    }
}

#[cfg(test)]
#[path = "facts_tests.rs"]
mod tests;
