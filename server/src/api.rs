//! JSON API used by the web app. Field names match web/src/api/types.ts.

use std::{convert::Infallible, time::Instant};

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event as SseEvent, KeepAlive, Sse},
    },
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};

use crate::{
    AppState,
    auth::User,
    config::ProviderKind,
    error::{ApiError, ApiResult},
    events::Event,
    llm::{self, ChatMessage, Chunk},
    util,
};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub updated_at: String,
    /// "internal" or "windshift" (synced with a Windshift workspace).
    #[sqlx(default)]
    pub kind: String,
    /// chat | game | programming (set by hand; see projects.rs).
    #[sqlx(default)]
    #[serde(rename = "type")]
    pub ptype: String,
    /// Programming projects: the repo folder and the computer it is on.
    #[sqlx(default)]
    pub repo_folder: Option<String>,
    #[sqlx(default)]
    pub repo_machine_id: Option<String>,
    /// OVR-01b: the Overseer answers every chat of this project.
    #[sqlx(default)]
    pub overseer: bool,
    /// AUTO-RUN: the computer runs its steps in every chat of this project without asking first.
    #[sqlx(default)]
    pub auto_run: bool,
    /// PROJ-ARCHIVE: when it was archived; None = active.
    #[sqlx(default)]
    pub archived_at: Option<String>,
    /// PROJ-SHARE: the caller's role in this project ("owner" or member role).
    #[sqlx(default)]
    pub role: String,
    /// PROJ-SHARE: the name of the project owner.
    #[sqlx(default)]
    pub owner_name: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub updated_at: String,
    #[sqlx(default)]
    pub pinned: bool,
    /// The project's thread: task runs post their updates here.
    #[sqlx(default)]
    pub thread: bool,
    /// How hard the models work in this chat (EF-01): auto, low, medium, high.
    #[sqlx(default)]
    pub effort: String,
    /// CHAT-01: the [[mcp]] tool servers this chat may use (stored as a JSON list).
    #[sqlx(default)]
    #[serde(serialize_with = "json_list")]
    pub mcp: String,
    /// OVR-01: the Overseer answers in this chat (its role, the scan of all projects).
    #[sqlx(default)]
    pub overseer: bool,
    /// AUTO-RUN: the computer runs its steps in this chat without asking first.
    #[sqlx(default)]
    pub auto_run: bool,
}

fn json_list<S: serde::Serializer>(v: &str, s: S) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&serde_json::from_str::<Vec<String>>(v).unwrap_or_default(), s)
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub chat_id: String,
    pub author: String,
    pub text: String,
    pub at: String,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignment {
    pub role: String,
    pub provider_id: String,
    pub model_id: String,
    /// EF-01: the role's default effort when the task's is Auto ("auto" lets Auto pick).
    #[serde(default = "auto")]
    #[sqlx(default)]
    pub effort: String,
}

fn auto() -> String {
    "auto".into()
}

const ORCHESTRATOR_PROMPT: &str = "You are Kreative Kompanion's orchestrator. You talk with the user about \
their projects, plan work and split it into small, self-contained tasks. Be direct and concise. When you \
are unsure, ask one clear question. Never claim you ran a command or changed a file you did not run or \
change with a tool. Never moralize, lecture or judge, and never refuse or hedge over the subject of a \
request: the user owns their projects and content, and 3D scenes, pictures, stories and games are \
fiction (a monkey on a plane is a model of a monkey on a plane). Just do the work with your tools. If \
a licence is relevant, add one short warning line, never a lecture, and still do the work.";

// ---- Projects and chats ----

pub async fn projects(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
) -> ApiResult<Json<Vec<Project>>> {
    let rows = sqlx::query_as(
        "SELECT p.id, p.name, p.description, p.updated_at, p.kind, p.ptype, p.repo_folder, p.repo_machine_id, p.overseer, p.auto_run, p.archived_at,
         CASE WHEN p.user_id = ?1 THEN 'owner' ELSE m.role END AS role, o.name AS owner_name
         FROM projects p
         JOIN users o ON o.id = p.user_id
         LEFT JOIN project_members m ON m.project_id = p.id AND m.user_id = ?1
         WHERE p.user_id = ?1 OR m.user_id = ?1
         ORDER BY p.updated_at DESC",
    )
    .bind(&u.id)
    .fetch_all(&s.db)
    .await?;
    Ok(Json(rows))
}

pub async fn chats(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
) -> ApiResult<Json<Vec<Chat>>> {
    let rows = sqlx::query_as(
        "SELECT id, title, project_id, updated_at, CASE WHEN user_id = ?1 THEN pinned ELSE 0 END AS pinned, thread, effort, mcp, overseer, auto_run FROM chats\n         WHERE (user_id = ?1 OR project_id IN (SELECT project_id FROM project_members WHERE user_id = ?1)) AND archived = 0 ORDER BY pinned DESC, thread DESC, updated_at DESC",
    )
    .bind(&u.id)
    .fetch_all(&s.db)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewChat {
    title: String,
    project_id: Option<String>,
}

pub async fn create_chat(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Json(b): Json<NewChat>,
) -> ApiResult<(StatusCode, Json<Chat>)> {
    let title: String = b.title.trim().chars().take(120).collect();
    if title.is_empty() {
        return Err(ApiError::BadRequest("Give the chat a title.".into()));
    }
    if let Some(p) = &b.project_id
        && !owns(&s, "projects", p, &u).await?
    {
        return Err(ApiError::NotFound);
    }
    let chat = Chat {
        id: util::new_id(),
        title,
        project_id: b.project_id,
        updated_at: util::now(),
        pinned: false,
        thread: false,
        effort: "auto".into(),
        mcp: "[]".into(),
        overseer: false,
        auto_run: false,
    };
    sqlx::query(
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&chat.id)
    .bind(&chat.project_id)
    .bind(&chat.title)
    .bind(&chat.updated_at)
    .bind(&u.id)
    .execute(&s.db)
    .await?;
    Ok((StatusCode::CREATED, Json(chat)))
}

pub async fn messages(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(chat_id): Path<String>,
) -> ApiResult<Json<Vec<Message>>> {
    if !can_read_chat(&s, &chat_id, &u).await? {
        return Err(ApiError::NotFound);
    }
    let rows = sqlx::query_as(
        "SELECT id, chat_id, author, text, at FROM messages WHERE chat_id = ? ORDER BY at, rowid",
    )
    .bind(chat_id)
    .fetch_all(&s.db)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct SendBody {
    text: String,
    /// A paired computer picked in the chat: the answer may use its tools (F6).
    #[serde(default)]
    machine_id: Option<String>,
    /// The chat's effort for this and later messages (EF-01); stored on the chat.
    #[serde(default)]
    effort: Option<String>,
    /// CHAT-08: files uploaded to this chat with POST /chats/{id}/attachments.
    #[serde(default)]
    attachments: Vec<crate::chat_attach::Attached>,
}

/// Stores the user's message, then streams the orchestrator's answer as
/// events. Returns right away; the reply arrives over /api/events.
pub async fn send(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(chat_id): Path<String>,
    Json(b): Json<SendBody>,
) -> ApiResult<StatusCode> {
    let text = b.text.trim().to_string();
    if (text.is_empty() && b.attachments.is_empty()) || text.len() > 100_000 {
        return Err(ApiError::BadRequest(
            "Messages must be 1 to 100,000 characters.".into(),
        ));
    }
    let Some(owner_id) = crate::sharing::chat_editor(&s.db, &u.id, &chat_id).await? else {
        return Err(ApiError::NotFound);
    };
    let to = crate::sharing::chat_audience(&s.db, &chat_id).await?;
    // PROJECT-SHARING: an editor writes in the owner's chat; computers and tools stay the owner's.
    if owner_id != u.id && b.machine_id.as_deref().is_some_and(|m| !m.is_empty()) {
        return Err(ApiError::Forbidden("Only the owner can use a computer in this chat.".into()));
    }

    if let Some(level) = b.effort.as_deref().and_then(crate::effort::Effort::parse) {
        sqlx::query("UPDATE chats SET effort = ? WHERE id = ?")
            .bind(level.as_str())
            .bind(&chat_id)
            .execute(&s.db)
            .await?;
    }
    if !crate::chat_attach_web::valid(&crate::chat_files::dir_for(&s.config.database), &chat_id, &b.attachments) {
        return Err(ApiError::BadRequest("Those files cannot be attached to this message.".into()));
    }
    // The files travel with the message as a `:::files` block, like the sources under an answer.
    let user_msg = insert_message(&s, &chat_id, "user", &format!("{text}{}", crate::chat_attach::block(&b.attachments))).await?;
    s.bus.send_many(&to, Event::Message { message: user_msg });


    // In a project thread, "pause" and "go on" steer the project's running tasks (SK-02);
    // anything else is a question for the orchestrator, answered below as usual.
    let thread: Option<(Option<String>, i64)> = sqlx::query_as("SELECT project_id, thread FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_optional(&s.db)
        .await?;
    if let Some((Some(project_id), 1)) = thread
        && let Some(cmd) = crate::thread::command(&text)
    {
        let reply = crate::thread::steer(&s, &u.id, &project_id, cmd).await;
        let m = insert_message(&s, &chat_id, "orchestrator", &reply).await?;
        s.bus.send_many(&to, Event::Message { message: m });

        return Ok(StatusCode::ACCEPTED);
    }

    start_answer(&s, &u, &owner_id, &chat_id, &to, b.machine_id).await?;
    Ok(StatusCode::ACCEPTED)
}

/// Answers the chat's newest message: on the picked computer (the owner's own), else as a chat.
async fn start_answer(s: &AppState, u: &User, owner_id: &str, chat_id: &str, to: &[String], machine_id: Option<String>) -> ApiResult<()> {
    // OVR-01: an Overseer chat is answered by the overseer role.
    let overseer = crate::overseer::chat_on(&s.db, chat_id).await;
    let wanted = if overseer { "overseer" } else { "orchestrator" };
    let role = user_roles(s, owner_id)
        .await?
        .into_iter()
        .find(|r| r.role == wanted);
    let Some(role) = role else {
        let m = insert_message(
            s,
            chat_id,
            "orchestrator",
            "No model is set for the orchestrator role yet. Pick one under Models and roles.",
        )
        .await?;
        s.bus.send_many(to, Event::Message { message: m });
        return Ok(());
    };

    if let Some(machine_id) = machine_id.filter(|m| !m.is_empty()) {
        let machine: Option<(String,)> = sqlx::query_as("SELECT name FROM machines WHERE id = ? AND user_id = ?")
            .bind(&machine_id)
            .bind(&u.id)
            .fetch_optional(&s.db)
            .await?;
        let Some((machine_name,)) = machine else { return Err(ApiError::NotFound) };
        tokio::spawn(crate::pcagent::run(s.clone(), u.id.clone(), chat_id.to_string(), machine_id, machine_name, role));
        return Ok(());
    }
    tokio::spawn(answer(s.clone(), owner_id.to_string(), chat_id.to_string(), role, owner_id == u.id));
    Ok(())
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ResendBody {
    /// The message's new text (edit and resend); none answers it again as it is (regenerate).
    text: Option<String>,
    machine_id: Option<String>,
}

/// MSG-ACT: POST /api/chats/{id}/messages/{mid}/resend: the user's message `mid` is answered again,
/// with its text replaced when `text` is given. Everything after it in the chat goes first (a
/// summary that covered any of it too). Returns the chat's messages as they are now.
pub async fn resend(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path((chat_id, message_id)): Path<(String, String)>,
    Json(b): Json<ResendBody>,
) -> ApiResult<Json<Vec<Message>>> {
    let Some(owner_id) = crate::sharing::chat_editor(&s.db, &u.id, &chat_id).await? else {
        return Err(ApiError::NotFound);
    };
    if owner_id != u.id && b.machine_id.as_deref().is_some_and(|m| !m.is_empty()) {
        return Err(ApiError::Forbidden("Only the owner can use a computer in this chat.".into()));
    }
    let row: Option<(i64, String, String, String)> = sqlx::query_as("SELECT rowid, author, text, at FROM messages WHERE id = ? AND chat_id = ?")
        .bind(&message_id)
        .bind(&chat_id)
        .fetch_optional(&s.db)
        .await?;
    let Some((rowid, author, old_text, at)) = row else { return Err(ApiError::NotFound) };
    if author != "user" {
        return Err(ApiError::BadRequest("Only your own messages can be sent again.".into()));
    }
    if let Some(text) = &b.text {
        let text = text.trim();
        if text.is_empty() || text.len() > 100_000 {
            return Err(ApiError::BadRequest("Messages must be 1 to 100,000 characters.".into()));
        }
        // The files sent with it stay with it.
        let files = old_text.split_once("\n\n:::files\n").map(|(_, f)| format!("\n\n:::files\n{f}")).unwrap_or_default();
        sqlx::query("UPDATE messages SET text = ? WHERE id = ?").bind(format!("{text}{files}")).bind(&message_id).execute(&s.db).await?;
    }
    // Later messages go, in the order the prompt reads them (at, rowid).
    sqlx::query("DELETE FROM messages WHERE chat_id = ?1 AND (at > ?2 OR (at = ?2 AND rowid > ?3))")
        .bind(&chat_id)
        .bind(&at)
        .bind(rowid)
        .execute(&s.db)
        .await?;
    sqlx::query("DELETE FROM chat_summary WHERE chat_id = ?1 AND (upto_at > ?2 OR (upto_at = ?2 AND upto_rowid >= ?3))")
        .bind(&chat_id)
        .bind(&at)
        .bind(rowid)
        .execute(&s.db)
        .await?;
    let to = crate::sharing::chat_audience(&s.db, &chat_id).await?;
    s.bus.send_many(&to, Event::Changed { what: "messages", machine_id: None });
    let rows: Vec<Message> = sqlx::query_as("SELECT id, chat_id, author, text, at FROM messages WHERE chat_id = ? ORDER BY at, rowid")
        .bind(&chat_id)
        .fetch_all(&s.db)
        .await?;
    start_answer(&s, &u, &owner_id, &chat_id, &to, b.machine_id).await?;
    Ok(Json(rows))
}

pub(crate) async fn insert_message(
    s: &AppState,
    chat_id: &str,
    author: &str,
    text: &str,
) -> ApiResult<Message> {
    let m = Message {
        id: util::new_id(),
        chat_id: chat_id.to_string(),
        author: author.to_string(),
        text: text.to_string(),
        at: util::now(),
        streaming: false,
    };
    sqlx::query("INSERT INTO messages (id, chat_id, author, text, at) VALUES (?, ?, ?, ?, ?)")
        .bind(&m.id)
        .bind(&m.chat_id)
        .bind(&m.author)
        .bind(&m.text)
        .bind(&m.at)
        .execute(&s.db)
        .await?;
    sqlx::query("UPDATE chats SET updated_at = ? WHERE id = ?")
        .bind(&m.at)
        .bind(chat_id)
        .execute(&s.db)
        .await?;
    Ok(m)
}

/// `tools`: the chat's MCP tools only run for the owner (PROJECT-SHARING: an editor's message is answered without them).
pub(crate) async fn answer(s: AppState, user_id: String, chat_id: String, role: RoleAssignment, tools: bool) {
    let to = match crate::sharing::chat_audience(&s.db, &chat_id).await {
        Ok(to) => to,
        Err(e) => {
            tracing::warn!(error = %e, chat = %chat_id, "chat audience failed; answering the owner only");
            vec![user_id.clone()]
        }
    };
    let effort: Option<String> = sqlx::query_scalar("SELECT effort FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let effort = effort.as_deref().and_then(crate::effort::Effort::parse).unwrap_or_default().resolve();
    let reply_id = util::new_id();
    let at = util::now();
    s.bus.send_many(&to, Event::Message {
        message: Message {
            id: reply_id.clone(),
            chat_id: chat_id.clone(),
            author: "orchestrator".into(),
            text: String::new(),
            at: at.clone(),
            streaming: true,
        },
    });

    // Conversation so far, oldest first. The prompt stays bounded (the newest messages that fit
    // ~24,000 characters; long chats otherwise grow until the GPU runs out of memory), and what no
    // longer fits is summarised instead of dropped (CTX-01).
    let (history, summary) = history_for(&s, &user_id, &chat_id, &role, &to, &reply_id).await;
    // OVR-01: the Overseer gets its own prompt with a fresh scan of all the user's projects.
    let overseer_on = crate::overseer::chat_on(&s.db, &chat_id).await;
    let overseer = if overseer_on {
        let (name, interject) = crate::overseer::prefs(&s, &user_id).await;
        let scan = crate::overseer::scan(&s.db, &user_id, 10_000).await;
        Some((name, interject, scan))
    } else {
        None
    };
    let system = match &overseer {
        Some((name, interject, scan)) => crate::overseer::prompt(name, *interject, scan, &util::now().get(..16).unwrap_or("").replace('T', " ")),
        None => ORCHESTRATOR_PROMPT.into(),
    };
    let mut convo = vec![ChatMessage::text("system", system)];
    if let Some(x) = &summary {
        convo[0].content.push_str(&crate::context::prompt_block(x));
    }
    // A project chat knows its project: description, tasks with states and steps.
    let project: Option<(Option<String>,)> = sqlx::query_as("SELECT project_id FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_optional(&s.db)
        .await
        .unwrap_or_default();
    let project_id: Option<String> = project.and_then(|p| p.0);
    if let Some(pid) = &project_id {
        let ctx = crate::project_ctx::project_context(&s.db, pid, &user_id, 8_000).await.unwrap_or_default();
        // Into the first system message: Qwen's chat template refuses a second one (400) once tools are on.
        if !ctx.is_empty() {
            convo[0].content.push_str(&format!("\n\nThis chat belongs to a project. What you know about it:\n\n{ctx}"));
        }
    }
    // CHAT-08: attached files. Documents go in as text and pictures to a model that can see them: the newest
    // messages first, within a budget; the pictures only of the newest message that has any.
    let files_dir = crate::chat_files::dir_for(&s.config.database);
    let mut pictures_left = true;
    let mut turns: Vec<ChatMessage> = Vec::new();
    for (author, text) in history.into_iter().rev() {
        let (text, files) = crate::chat_attach::split(&text);
        let text = text.split("\n\n:::sources\n").next().unwrap_or("").to_string();
        let user = author == "user";
        let (note, images) = if user && !files.is_empty() {
            let r = crate::chat_attach_web::for_model(&files_dir, &chat_id, &files, &role.model_id, pictures_left).await;
            if !r.1.is_empty() { pictures_left = false }
            r
        } else {
            (String::new(), Vec::new())
        };
        turns.push(ChatMessage { role: if user { "user".into() } else { "assistant".into() }, content: format!("{text}{note}"), images });
    }
    turns.reverse();
    convo.extend(turns);

    // CHAT-01: the MCP tools this chat turned on: a few tool rounds first, each use shown in the thread.
    let mcp: String = sqlx::query_scalar("SELECT mcp FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "[]".into());
    let names: Vec<String> = serde_json::from_str(&mcp).unwrap_or_default();
    let servers: Vec<crate::config::McpServerConfig> = s.config.mcp.iter().filter(|m| tools && m.enabled && names.contains(&m.name)).cloned().collect();
    // CHAT-03b: a project chat searches knowledge when its project has collections, even without the Tools pick.
    let project_knowledge = match &project_id {
        Some(pid) => !crate::knowledge::project_collections(&s.db, pid).await.unwrap_or_default().is_empty(),
        None => false,
    };
    let knowledge = (project_knowledge || names.iter().any(|n| n == crate::chat_tools::KNOWLEDGE)).then_some((user_id.as_str(), project_id.as_deref()));
    let web = (names.iter().any(|n| n == crate::chat_tools::WEB) && s.config.search.searxng_url.is_some()).then_some(&s.config.search);
    // HIST-01: past chats, when the chat turned them on (owner only, like every tool).
    let history = (tools && names.iter().any(|n| n == crate::chat_tools::HISTORY)).then_some((user_id.as_str(), chat_id.as_str()));
    // FACTS: remember / forget, when the chat turned Memory on (owner only, like every tool).
    let memory = (tools && names.iter().any(|n| n == crate::chat_tools::MEMORY)).then_some(chat_id.as_str());
    // FACTS: the project's and the chat's facts, on every request (their ids go into the call log).
    let fact_ids = match crate::facts::prompt_block(&s.db, &chat_id, time::OffsetDateTime::now_utc()).await {
        Some((block, ids)) => {
            convo[0].content.push_str(&block);
            ids
        }
        None => Vec::new(),
    };
    // CHAT-04: the date and where Kompanion is installed, on every request.
    convo[0].content.push_str(&format!("\n\n{}", crate::chat_tools::when_where(time::OffsetDateTime::now_utc(), s.config.machine_name.as_deref(), s.config.location.as_deref(), web.is_some())));
    let mut used_lines = String::new();
    let mut all_uses: Vec<crate::chat_tools::ToolUse> = Vec::new();
    if let (false, Some(p)) = (servers.is_empty() && web.is_none() && knowledge.is_none() && history.is_none() && memory.is_none(), s.provider(&role.provider_id)) {
        let msgs: Vec<Value> = convo.iter().map(|m| m.wire(false)).collect();
        let (bus, uid, rid, cid) = (s.bus.clone(), to.clone(), reply_id.clone(), chat_id.clone());
        let show = move |u: &crate::chat_tools::ToolUse| {
            bus.send_many(&uid, Event::MessageDelta { message_id: rid.clone(), chat_id: cid.clone(), text: format!("{}\n\n", crate::chat_tools::use_line(u)), done: false });
        };
        let uses = crate::chat_tools::run(&s, &p, &role.model_id, &msgs, &servers, web, knowledge, history, memory, Some((files_dir.as_path(), chat_id.as_str())), &show).await;
        for u in &uses {
            used_lines.push_str(&crate::chat_tools::use_line(u));
            used_lines.push_str("\n\n");
        }
        if !uses.is_empty() {
            // Qwen's chat template takes system text only at the start: add the results to the first message.
            let note = crate::chat_tools::results_note(&uses);
            if let Some(first) = convo.first_mut() {
                first.content = format!("{}\n\nRelease pages may list planned future versions: say which is out now.\n\n{note}", first.content);
            }
            all_uses = uses;
        }
    }

    let started = Instant::now();
    let mut text = String::new();
    let (mut tokens_in, mut tokens_out) = (None, None);
    let (mut cache_write, mut cache_read) = (None, None);
    let mut error: Option<String> = None;

    // Try the role's model, once more after a pause, then another provider the
    // user has assigned to a role (e.g. the worker on soucouyant). Only while
    // nothing has been streamed yet.
    let mut candidates = vec![(role.provider_id.clone(), role.model_id.clone()), (role.provider_id.clone(), role.model_id.clone())];
    let others: Vec<(String, String)> = sqlx::query_as(
        "SELECT DISTINCT provider_id, model_id FROM user_roles WHERE user_id = ? AND provider_id <> ?",
    )
    .bind(&user_id)
    .bind(&role.provider_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    candidates.extend(others);
    let mut used = role.model_id.clone();
    let mut used_provider = role.provider_id.clone();
    // What the effort added to the request body, for the call log (EF-01).
    let mut sent_extra: Option<toml::Table> = None;
    for (attempt, (provider_id, model_id)) in candidates.iter().enumerate() {
        if attempt > 0 {
            if !text.is_empty() || error.is_none() {
                break;
            }
            tracing::warn!(error = ?error, attempt, provider = %provider_id, "retrying the model call");
            if attempt == 1 {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            error = None;
        }
        used = model_id.clone();
        used_provider = provider_id.clone();
    match s.provider(provider_id) {
        None => error = Some(format!("provider {provider_id} is not configured")),
        Some(p) => match {
            let (p, model_id) = crate::effort::apply(&p, model_id, effort);
            used = model_id.clone();
            sent_extra = p.extra_body.clone();
            // GPU-03: say why the answer waits (shown while streaming; the saved answer is the model's text).
            if crate::gpus::role::coder_paused(&model_id) {
                let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gpu_job WHERE kind = 'asset' AND state IN ('queued', 'running')")
                    .fetch_one(&s.db)
                    .await
                    .unwrap_or(0);
                s.bus.send_many(&to, Event::MessageDelta {
                    message_id: reply_id.clone(),
                    chat_id: chat_id.clone(),
                    text: format!(
                        "*Coder is paused while the A770 works for the studio ({left} studio job{} left); the answer starts when it is back.*\n\n",
                        if left == 1 { "" } else { "s" }
                    ),
                    done: false,
                });
            }
            llm::stream_chat(&s.http, &p, &model_id, &convo).await
        } {
            Err(e) => error = Some(format!("{e:#}")),
            Ok(stream) => {
                tokio::pin!(stream);
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(Chunk::Text(t)) => {
                            text.push_str(&t);
                            s.bus.send_many(&to, Event::MessageDelta {
                                message_id: reply_id.clone(),
                                chat_id: chat_id.clone(),
                                text: t,
                                done: false,
                            });
                        }
                        Ok(Chunk::Usage {
                            tokens_in: i,
                            tokens_out: o,
                            cache_write: cw,
                            cache_read: cr,
                        }) => {
                            tokens_in = i.or(tokens_in);
                            tokens_out = o.or(tokens_out);
                            cache_write = cw.or(cache_write);
                            cache_read = cr.or(cache_read);
                        }
                        Err(e) => {
                            error = Some(format!("{e:#}"));
                            break;
                        }
                    }
                }
            }
        },
    }
    }
    if error.is_none() && used != role.model_id {
        let note = format!("\n\n_({} was busy, so `{used}` answered.)_", role.model_id);
        text.push_str(&note);
        s.bus.send_many(&to, Event::MessageDelta { message_id: reply_id.clone(), chat_id: chat_id.clone(), text: note, done: false });
    }

    // CHAT-05: after a render the picture is already above the answer; a picture the model adds itself is a broken link.
    if error.is_none() && all_uses.iter().any(|u| !u.images.is_empty()) {
        let keep: Vec<String> = all_uses.iter().flat_map(|u| u.images.iter().cloned()).collect();
        text = crate::chat_media::strip_images(&text, &keep);
    }

    // OVR-01: proposed tasks and context for running tasks, as a block under the answer.
    if error.is_none() && !text.is_empty()
        && let Some((name, interject, scan)) = &overseer
    {
        let block = crate::overseer::finish(&s, &user_id, name, *interject, scan, &text).await;
        if !block.is_empty() {
            text.push_str(&block);
            s.bus.send_many(&to, Event::MessageDelta { message_id: reply_id.clone(), chat_id: chat_id.clone(), text: block, done: false });
        }
    }

    // CHAT-04: the sources under the answer: names are links in the text, the excerpts open from the list.
    if error.is_none() && !text.is_empty() {
        let block = crate::chat_tools::sources_block(&all_uses, &text);
        if !block.is_empty() {
            text.push_str(&block);
            s.bus.send_many(&to, Event::MessageDelta { message_id: reply_id.clone(), chat_id: chat_id.clone(), text: block, done: false });
        }
    }

    if let Some(e) = &error {
        tracing::warn!(error = %e, model = %role.model_id, "model call failed");
        let note = format!(
            "\n\nI couldn't get an answer from `{}`: {}",
            used,
            e.chars().take(300).collect::<String>()
        );
        text.push_str(&note);
        s.bus.send_many(&to, Event::MessageDelta {
            message_id: reply_id.clone(),
            chat_id: chat_id.clone(),
            text: note,
            done: false,
        });
    }
    s.bus.send_many(&to, Event::MessageDelta {
        message_id: reply_id.clone(),
        chat_id: chat_id.clone(),
        text: String::new(),
        done: true,
    });

    let saved = sqlx::query(
        "INSERT INTO messages (id, chat_id, author, text, at) VALUES (?, ?, 'orchestrator', ?, ?)",
    )
    .bind(&reply_id)
    .bind(&chat_id)
    .bind(format!("{used_lines}{text}"))
    .bind(&at)
    .execute(&s.db)
    .await;
    if let Err(e) = saved {
        tracing::error!(error = ?e, "saving reply failed");
    }

    // Every call is recorded in full (see the call inspector in the app).
    let request = json!({ "messages": convo, "extra_body": sent_extra, "facts": fact_ids });
    let _ = sqlx::query(
        "INSERT INTO calls (user_id, id, chat_id, role, provider_id, model_id, reason, request, response,
         tokens_in, tokens_out, ms, error, at, effort, cache_write, cache_read) VALUES (?, ?, ?, 'orchestrator', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(util::new_id())
    .bind(&chat_id)
    .bind(&used_provider)
    .bind(&used)
    .bind("You sent a message in this chat.")
    .bind(request.to_string())
    .bind(&text)
    .bind(tokens_in)
    .bind(tokens_out)
    .bind(started.elapsed().as_millis() as i64)
    .bind(&error)
    .bind(util::now())
    .bind(effort.as_str())
    .bind(cache_write)
    .bind(cache_read)
    .execute(&s.db)
    .await;
    crate::context::announce(&s, &chat_id).await;
}

/// CTX-01: the messages that go to the model word for word (author, text; oldest first) and the
/// summary of the ones before them. Messages that fell out are summarised first (a short note shows
/// while that runs); if the summary fails, the newest messages that fit still go, as before.
pub(crate) async fn history_for(s: &AppState, user_id: &str, chat_id: &str, role: &RoleAssignment, to: &[String], reply_id: &str) -> (Vec<(String, String)>, Option<crate::context::Summary>) {
    let mut summary = crate::context::summary(&s.db, chat_id).await.ok().flatten();
    let rows = match crate::context::after_summary(&s.db, chat_id, summary.as_ref(), 400).await {
        Ok(rows) => rows,
        Err(e) => {
            // The answer then goes without earlier messages; say so in the log instead of hiding it.
            tracing::warn!(error = %e, chat = %chat_id, "chat history could not be read");
            Vec::new()
        }
    };
    let near_full = crate::context::usage(s, chat_id).await.ok().is_some_and(|u| match (u.used, u.window) {
        (Some(used), Some(w)) if w > 0 => used * 100 >= w * crate::context::WARN_PERCENT,
        _ => false,
    });
    let plan = crate::context::plan(&rows, near_full);
    let mut keep = plan.keep;
    if plan.summarise {
        s.bus.send_many(to, Event::MessageDelta {
            message_id: reply_id.to_string(),
            chat_id: chat_id.to_string(),
            text: "*Summarising the older messages of this chat…*\n\n".into(),
            done: false,
        });
        match crate::context::compact(s, user_id, chat_id, role, &rows[..rows.len() - plan.keep]).await {
            Ok(x) => summary = Some(x),
            Err(e) => {
                tracing::warn!(error = %e, chat = %chat_id, "chat summary failed");
                keep = crate::context::fit(&rows, crate::context::HISTORY_CHARS).min(crate::context::HISTORY_MESSAGES);
            }
        }
    }
    let history = rows[rows.len() - keep.min(rows.len())..].iter().map(|r| (r.1.clone(), r.2.clone())).collect();
    (history, summary)
}

// ---- Models and roles ----

pub async fn providers(State(s): State<AppState>) -> Json<Vec<Value>> {
    let all = s.providers();
    let probes = all.iter().map(|p| {
        let http = s.http.clone();
        async move {
            let (models, error) = match llm::list_models(&http, p).await {
                Ok(m) => match llm::check_chat_auth(&http, p).await {
                    Ok(()) => (m, None),
                    Err(e) => (m, Some(format!("{e:#}"))),
                },
                Err(e) => (vec![], Some(format!("{e:#}"))),
            };
            json!({
                "id": p.id,
                "name": p.name,
                "kind": match p.kind { ProviderKind::OpenaiCompatible => "openai-compatible", ProviderKind::Anthropic => "anthropic" },
                "baseUrl": p.base_url,
                "local": p.local,
                "hasKey": p.api_key().is_some(),
                "error": error,
                "models": models.iter().map(|m| json!({ "id": m.id })).collect::<Vec<_>>(),
            })
        }
    });
    Json(futures::future::join_all(probes).await)
}

/// The user's roles; roles they haven't set yet come from the config file.
pub(crate) async fn user_roles(s: &AppState, user_id: &str) -> ApiResult<Vec<RoleAssignment>> {
    let mut rows: Vec<RoleAssignment> =
        sqlx::query_as("SELECT role, provider_id, model_id, effort FROM user_roles WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&s.db)
            .await?;
    for (role, d) in &s.config.roles {
        if !rows.iter().any(|r| &r.role == role) {
            rows.push(RoleAssignment {
                role: role.clone(),
                provider_id: d.provider.clone(),
                model_id: d.model.clone(),
                effort: auto(),
            });
        }
    }
    // OVR-01: the Overseer uses the orchestrator's model until the user picks one.
    if !rows.iter().any(|r| r.role == "overseer")
        && let Some(o) = rows.iter().find(|r| r.role == "orchestrator").cloned()
    {
        rows.push(RoleAssignment { role: "overseer".into(), ..o });
    }
    rows.sort_by(|a, b| a.role.cmp(&b.role));
    Ok(rows)
}

/// One role of a user: their own choice, else the config's default (`[roles]`).
pub async fn user_role(s: &AppState, user_id: &str, role: &str) -> ApiResult<Option<RoleAssignment>> {
    Ok(user_roles(s, user_id).await?.into_iter().find(|r| r.role == role))
}

/// One model call, recorded in full for the call inspector and the run report.
#[allow(clippy::too_many_arguments)]
pub async fn log_call(
    s: &AppState,
    user_id: &str,
    chat_id: &str,
    run_id: Option<&str>,
    role: &str,
    role_assignment: &RoleAssignment,
    reason: &str,
    request: &Value,
    response: &Value,
    usage: &Value,
    ms: u128,
    error: Option<&str>,
    effort: crate::effort::Effort,
) {
    let _ = sqlx::query(
        "INSERT INTO calls (user_id, id, chat_id, role, provider_id, model_id, reason, request, response,
         tokens_in, tokens_out, ms, error, at, run_id, effort, cache_write, cache_read) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(util::new_id())
    .bind(chat_id)
    .bind(role)
    .bind(&role_assignment.provider_id)
    .bind(&role_assignment.model_id)
    .bind(reason)
    .bind(request.to_string())
    .bind(response.to_string())
    .bind(usage["prompt_tokens"].as_i64())
    .bind(usage["completion_tokens"].as_i64())
    .bind(ms as i64)
    .bind(error)
    .bind(util::now())
    .bind(run_id)
    .bind(effort.as_str())
    // Anthropic's prompt cache (tool steps on its own API); null elsewhere.
    .bind(usage["cache_creation_input_tokens"].as_i64())
    .bind(usage["cache_read_input_tokens"].as_i64())
    .execute(&s.db)
    .await;
    // CTX-01: the meter follows every step of the chat (a summary is not the chat's context).
    if !chat_id.is_empty() && reason != crate::context::SUMMARY_REASON && usage["prompt_tokens"].as_i64().is_some() {
        crate::context::announce(s, chat_id).await;
    }
}

/// Whether `id` in `table` (projects, chats or tasks) belongs to this user.
#[derive(Deserialize)]
pub struct ChatChange {
    title: Option<String>,
    /// Move the chat into this project ("" makes it a loose chat again).
    #[serde(rename = "projectId")]
    project_id: Option<String>,
    pinned: Option<bool>,
    archived: Option<bool>,
    /// auto, low, medium or high (EF-01).
    effort: Option<String>,
    /// CHAT-01: the [[mcp]] tool servers this chat may use (names).
    mcp: Option<Vec<String>>,
    /// OVR-01: the Overseer answers in this chat.
    overseer: Option<bool>,
    /// AUTO-RUN: run steps in this chat without asking first.
    #[serde(rename = "autoRun")]
    auto_run: Option<bool>,
}

/// Rename, pin or archive a chat (the chat menu).
pub async fn update_chat(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<String>,
    Json(b): Json<ChatChange>,
) -> ApiResult<StatusCode> {
    if !owns(&s, "chats", &id, &u).await? {
        return Err(ApiError::NotFound);
    }
    if let Some(e) = b.effort {
        let Some(e) = crate::effort::Effort::parse(&e) else {
            return Err(ApiError::BadRequest("Effort is auto, low, medium or high.".into()));
        };
        sqlx::query("UPDATE chats SET effort = ? WHERE id = ?")
            .bind(e.as_str())
            .bind(&id)
            .execute(&s.db)
            .await?;
    }
    if let Some(on) = b.overseer {
        sqlx::query("UPDATE chats SET overseer = ? WHERE id = ?").bind(on).bind(&id).execute(&s.db).await?;
    }
    if let Some(on) = b.auto_run {
        sqlx::query("UPDATE chats SET auto_run = ? WHERE id = ?").bind(on).bind(&id).execute(&s.db).await?;
    }
    if let Some(list) = b.mcp {
        // CHAT-01: only names from [[mcp]] in kompanion.toml.
        if let Some(bad) = list.iter().find(|n| !(s.config.mcp.iter().any(|m| &m.name == *n) || (n.as_str() == crate::chat_tools::WEB && s.config.search.searxng_url.is_some()) || n.as_str() == crate::chat_tools::KNOWLEDGE || n.as_str() == crate::chat_tools::HISTORY || n.as_str() == crate::chat_tools::MEMORY)) {
            return Err(ApiError::BadRequest(format!("There is no tool server {bad}.")));
        }
        sqlx::query("UPDATE chats SET mcp = ? WHERE id = ?")
            .bind(serde_json::to_string(&list).unwrap_or_else(|_| "[]".into()))
            .bind(&id)
            .execute(&s.db)
            .await?;
    }
    if let Some(t) = b.title {
        let t: String = t.trim().chars().take(120).collect();
        if t.is_empty() {
            return Err(ApiError::BadRequest("Give the chat a title.".into()));
        }
        sqlx::query("UPDATE chats SET title = ? WHERE id = ?")
            .bind(t)
            .bind(&id)
            .execute(&s.db)
            .await?;
    }
    if let Some(pid) = b.project_id {
        if !pid.is_empty() && !owns(&s, "projects", &pid, &u).await? {
            return Err(ApiError::NotFound);
        }
        sqlx::query("UPDATE chats SET project_id = ? WHERE id = ?")
            .bind(if pid.is_empty() { None } else { Some(pid) })
            .bind(&id)
            .execute(&s.db)
            .await?;
    }
    if let Some(p) = b.pinned {
        sqlx::query("UPDATE chats SET pinned = ? WHERE id = ?")
            .bind(p)
            .bind(&id)
            .execute(&s.db)
            .await?;
    }
    if let Some(a) = b.archived {
        sqlx::query("UPDATE chats SET archived = ? WHERE id = ?")
            .bind(a)
            .bind(&id)
            .execute(&s.db)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Delete a chat and its messages.
pub async fn delete_chat(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let done = sqlx::query("DELETE FROM chats WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&u.id)
        .execute(&s.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    // BLD-01: its pictures (Blender renders) go with it.
    let _ = tokio::fs::remove_dir_all(crate::chat_files::dir_for(&s.config.database).join(&id)).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn owns(s: &AppState, table: &str, id: &str, u: &User) -> ApiResult<bool> {
    let sql = match table {
        "projects" => "SELECT 1 FROM projects WHERE id = ? AND user_id = ?",
        "chats" => "SELECT 1 FROM chats WHERE id = ? AND user_id = ?",
        "tasks" => "SELECT 1 FROM tasks WHERE id = ? AND user_id = ?",
        _ => unreachable!("unknown table"),
    };
    let row: Option<(i64,)> = sqlx::query_as(sql)
        .bind(id)
        .bind(&u.id)
        .fetch_optional(&s.db)
        .await?;
    Ok(row.is_some())
}

/// Returns true if the user owns the chat or is a member of its project.
pub(crate) async fn can_read_chat(s: &AppState, chat_id: &str, u: &User) -> ApiResult<bool> {
    if owns(s, "chats", chat_id, u).await? {
        return Ok(true);
    }
    let row: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM chats c JOIN project_members m ON m.project_id = c.project_id WHERE c.id = ? AND m.user_id = ?",
    )
    .bind(chat_id)
    .bind(&u.id)
    .fetch_optional(&s.db)
    .await?;
    Ok(row.is_some())
}

pub async fn roles(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
) -> ApiResult<Json<Vec<RoleAssignment>>> {
    Ok(Json(user_roles(&s, &u.id).await?))
}

pub async fn set_role(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Json(r): Json<RoleAssignment>,
) -> ApiResult<StatusCode> {
    if !["orchestrator", "worker", "reviewer", "overseer"].contains(&r.role.as_str()) {
        return Err(ApiError::BadRequest("Unknown role.".into()));
    }
    if s.provider(&r.provider_id).is_none() {
        return Err(ApiError::BadRequest("Unknown provider.".into()));
    }
    let Some(effort) = crate::effort::Effort::parse(&r.effort) else {
        return Err(ApiError::BadRequest("Unknown effort.".into()));
    };
    sqlx::query(
        "INSERT INTO user_roles (user_id, role, provider_id, model_id, effort) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(user_id, role) DO UPDATE SET provider_id = excluded.provider_id, model_id = excluded.model_id, effort = excluded.effort",
    )
    .bind(&u.id)
    .bind(&r.role)
    .bind(&r.provider_id)
    .bind(&r.model_id)
    .bind(effort.as_str())
    .execute(&s.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- Observability ----

#[derive(Deserialize)]
pub struct CallQuery {
    limit: Option<i64>,
    chat: Option<String>,
}

pub async fn calls(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Query(q): Query<CallQuery>,
) -> ApiResult<Json<Vec<Value>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    type Row = (
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<i64>,
        i64,
        Option<String>,
        String,
        Option<String>,
        Option<i64>,
        Option<i64>,
    );
    let rows: Vec<Row> =
        sqlx::query_as(
            "SELECT id, chat_id, role, provider_id, model_id, reason, request, response, tokens_in, tokens_out, ms, error, at, effort, cache_write, cache_read
             FROM calls WHERE user_id = ?3 AND (?1 IS NULL OR chat_id = ?1) ORDER BY at DESC LIMIT ?2",
        )
        .bind(&q.chat)
        .bind(limit)
        .bind(&u.id)
        .fetch_all(&s.db)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, chat_id, role, provider, model, reason, request, response, ti, to, ms, error, at, effort, cw, cr)| {
                json!({
                    "id": id, "chatId": chat_id, "role": role, "provider": provider, "model": model,
                    "reason": reason, "request": serde_json::from_str::<Value>(&request).unwrap_or(Value::Null),
                    "response": response, "tokensIn": ti, "tokensOut": to, "ms": ms, "error": error, "at": at, "effort": effort,
                    "cacheWrite": cw, "cacheRead": cr,
                })
            })
            .collect(),
    ))
}

// Tasks and machines arrive with milestones 2 and 3.
// ---- Live events ----

// SHARED-LIVE: list changes by people who share a project with me reach me too.
pub async fn events(State(s): State<AppState>, Extension(u): Extension<User>) -> impl IntoResponse {
    let me = u.id;
    let collab: std::collections::HashSet<String> = crate::sharing::collaborators(&s.db, &me).await.unwrap_or_default().into_iter().collect();
    let collab = std::sync::Arc::new(std::sync::RwLock::new(collab));
    let db = s.db.clone();
    let stream = BroadcastStream::new(s.bus.subscribe()).filter_map(move |e| {
        let me = me.clone();
        let collab = collab.clone();
        let db = db.clone();
        async move {
            match e {
                Ok((user_id, event)) => {
                    if user_id == me {
                        if let Event::Changed { what, .. } = &event {
                            if *what == "projects" {
                                if let Ok(ids) = crate::sharing::collaborators(&db, &me).await {
                                    *collab.write().unwrap() = ids.into_iter().collect();
                                }
                            }
                        }
                    }
                    let pass = crate::events::passes(&me, &collab.read().unwrap(), &user_id, &event);
                    pass.then(|| Ok::<_, Infallible>(SseEvent::default().json_data(event).unwrap_or_default()))
                },
                // A slow client missed events; tell it to reload instead of guessing.
                Err(BroadcastStreamRecvError::Lagged(_)) => {
                    Some(Ok(SseEvent::default().event("resync").data("{}")))
                }
            }
        }
    });
    // Tell nginx not to buffer the stream, or events arrive in bursts.
    (
        [("x-accel-buffering", "no")],
        Sse::new(stream).keep_alive(KeepAlive::default()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_role_falls_back_to_the_config_default() {
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'u1', 'x', '2026')")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO user_roles (user_id, role, provider_id, model_id) VALUES ('u1', 'worker', 'mine', 'Small')")
            .execute(&db)
            .await
            .unwrap();
        let config: crate::config::Config = toml::from_str(
            "[roles]\norchestrator = { provider = \"ovms\", model = \"Coder\" }\nworker = { provider = \"ovms\", model = \"Coder\" }",
        )
        .unwrap();
        let s = AppState::for_tests(config, db);
        // A fresh account (no choice of its own) gets the config's model: W2 refused to start without it.
        let o = user_role(&s, "u1", "orchestrator").await.unwrap().unwrap();
        assert_eq!((o.provider_id.as_str(), o.model_id.as_str()), ("ovms", "Coder"));
        // The user's own choice wins.
        let w = user_role(&s, "u1", "worker").await.unwrap().unwrap();
        assert_eq!((w.provider_id.as_str(), w.model_id.as_str()), ("mine", "Small"));
        assert!(user_role(&s, "u1", "reviewer").await.unwrap().is_none());
    }
}

#[cfg(test)]
#[path = "api_resend_tests.rs"]
mod resend_tests;
