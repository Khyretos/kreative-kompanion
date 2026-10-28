//! The PC agent (F6): with a computer picked in the chat, the orchestrator model
//! gets the runner's tools. Every tool call waits for the user's decision in an
//! approval card (Approve, Always allow for 24 h, Deny); approved calls run as
//! runner jobs, which still check the computer's grants.
//! (The model drafted this twice; Claude rewrote it from the same spec.)
pub mod picture;
pub mod spawn;
pub mod steplog;
pub mod tools;

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{AppState, access, api::RoleAssignment, auth::User, error::{ApiError, ApiResult}, events::Event, llm, util};

/// Runner jobs of running steps: action id -> (machine id, job id), so Stop can cancel them.
static RUNNING_JOBS: LazyLock<std::sync::Mutex<HashMap<String, (String, String)>>> = LazyLock::new(Default::default);
/// Steps whose Stop button was really pressed. Only these may end as "stopped": the output
/// text alone can't decide it (a command that prints the runner's stop message would match).
static STOP_REQUESTED: LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> = LazyLock::new(Default::default);
/// Start of the result text for a step the user really stopped (set from the state, not from the output).
const STEP_STOPPED: &str = "The step was stopped by the user.";

const MAX_STEPS: usize = 8;
/// After this many cut-off answers in a row the agent stops instead of trying again.
const MAX_CUT_OFFS: usize = 3;
/// Told to the model when its answer hit the output limit before its tool call was complete.
const CUT_OFF_NOTE: &str = "Your last answer was cut off at the output limit before its tool call was complete, so nothing ran. \
    Do the same in smaller pieces: several edit_file calls that each replace a few lines, or write_file for one small file at a time.";
/// How long a card waits for a decision, and a job for the computer (seconds).
const WAIT_SECS: u32 = 1800;

fn system_prompt(machine: &str) -> String {
    format!(
        "You are Kreative Kompanion. You can act on the user's computer \"{machine}\" with the tools you have. \
         Every tool call is shown to the user, who approves or declines it, and the computer only allows what \
         was granted. Prefer small, safe steps: look before you change (read a file before editing it, search \
         before installing). Explain in one short sentence what you will do before calling a tool. When done, \
         say plainly what you changed. read_file takes offset and length for a part of a big file. Change files \
         with edit_file in small pieces (a few lines each) rather than rewriting a whole file; you may make several \
         calls in one answer. Once you have read what you need, make the change; read a file again only when it changed. \
         Never ask for passwords; the computer asks for them itself. \
         When the user refers to something said earlier or in another chat that you don't have, look it up \
         with search_chats and read_chat before saying you don't know. \
         Do every part of the request: after each result, call the next tool until all parts are done \
         (for \"install X and show its version\": install, then run the version command). Only say a step \
         worked when its result is \"done\" with \"exit: 0\", and quote the key output line. When a step \
         failed, was refused or declined, say so plainly and stop. \
         Never route around a refused or declined step with another tool (for example reading a refused \
         file with shell); report it instead. Never create a config or system file the user did not ask \
         you to create: if the file you were asked to change does not exist, stop, say what is there \
         (list the folder) and ask. Before changing desktop or app settings, call system_info and use the \
         config it reports (Hyprland: hyprland.lua or hyprland.conf, whichever exists). After a change, \
         check its effect with a read-only command (Hyprland: `hyprctl getoption general:gaps_out`) before \
         saying it worked; if you can't check, say it is not verified."
    )
}

/// Cuts text to at most `max` bytes on a character boundary.
fn cut(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

async fn say(s: &AppState, user_id: &str, chat_id: &str, text: &str) {
    match crate::api::insert_message(s, chat_id, "orchestrator", text).await {
        Ok(message) => s.bus.send(user_id, Event::Message { message }),
        Err(e) => tracing::warn!("pc agent: can't store a message: {e}"),
    }
}

fn changed(s: &AppState, user_id: &str) {
    s.bus.send(user_id, Event::Changed { what: "actions", machine_id: None });
}

/// The start of the error `Agent::run` returns when it used all its steps.
const STEP_LIMIT: &str = "I stopped after";

/// Whether an `Agent::run` error only means it ran out of steps (the work may well be done).
pub fn hit_step_limit(err: &str) -> bool {
    err.starts_with(STEP_LIMIT)
}

/// Stops a running step of `user_id`: the runner kills the command and its children
/// (runner 0.4.4 and newer). Ok(false) when the step isn't running.
pub async fn stop_action(s: &AppState, user_id: &str, action_id: &str) -> ApiResult<bool> {
    let mine: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM pc_actions WHERE id = ? AND user_id = ?")
        .bind(action_id)
        .bind(user_id)
        .fetch_optional(&s.db)
        .await?;
    if mine.is_none() {
        return Err(ApiError::NotFound);
    }
    let Some((machine_id, job_id)) = RUNNING_JOBS.lock().unwrap().get(action_id).cloned() else {
        return Ok(false);
    };
    STOP_REQUESTED.lock().unwrap().insert(action_id.to_string());
    access::queue_job(&s.db, &machine_id, user_id, &json!({ "tool": "cancel_job", "job": job_id }), None).await?;
    Ok(true)
}

/// Chats whose user pressed Stop: the loop checks this between steps (stopping a running step is separate).
static STOPPED_CHATS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> = std::sync::LazyLock::new(Default::default);

fn chat_stopped(chat_id: &str) -> bool {
    STOPPED_CHATS.lock().unwrap().contains(chat_id)
}

/// POST /chats/{id}/stop: ends the chat's agent at its next step and stops a step that is running now.
pub async fn stop_chat(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    if crate::sharing::chat_editor(&s.db, &u.id, &id).await?.is_none() {
        return Err(ApiError::NotFound);
    }
    STOPPED_CHATS.lock().unwrap().insert(id.clone());
    let running: Vec<(String,)> = sqlx::query_as("SELECT id FROM pc_actions WHERE chat_id = ? AND state = 'running'")
        .bind(&id)
        .fetch_all(&s.db)
        .await?;
    for (action,) in running {
        let _ = stop_action(&s, &u.id, &action).await;
    }
    Ok(StatusCode::ACCEPTED)
}

/// POST /actions/{id}/stop
pub async fn stop(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    if stop_action(&s, &u.id, &id).await? {
        Ok(StatusCode::ACCEPTED)
    } else {
        Err(ApiError::BadRequest("That step isn't running.".into()))
    }
}

/// AUTO-01: how many tool steps one request may take: the user's setting, kept between 1 and 100; 0 means no limit.
pub fn clamp_steps(n: i64) -> usize {
    if n == 0 { usize::MAX } else { n.clamp(1, 100) as usize }
}

/// AUTO-01: does this chat run steps without asking? Only when the chat's own toggle is on, or the
/// toggle of the project it belongs to is on. Off when nothing says otherwise.
async fn auto_run_for(s: &AppState, chat_id: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT c.auto_run = 1 OR COALESCE((SELECT p.auto_run = 1 FROM projects p WHERE p.id = c.project_id), 0)
         FROM chats c WHERE c.id = ?",
    )
    .bind(chat_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    .unwrap_or(false)
}

async fn max_steps_for(s: &AppState, user_id: &str) -> usize {
    let n: Option<i64> = sqlx::query_scalar("SELECT max_steps FROM users WHERE id = ?").bind(user_id).fetch_optional(&s.db).await.ok().flatten();
    n.map(clamp_steps).unwrap_or(MAX_STEPS)
}

/// What the agent did in a chat's last answer: the whole exchange with the model (its tool calls
/// and their results, not only the chat lines), so the next message goes on from there instead of
/// reading the same files again. Kept in memory: after a restart the chat lines are used.
struct Memory {
    provider_id: String,
    model_id: String,
    /// The conversation without the system prompt, ending with the agent's last answer.
    messages: Vec<Value>,
    /// The time of the chat's latest message when this was saved: newer ones are new.
    until: String,
}

static MEMORY: LazyLock<std::sync::Mutex<HashMap<String, Memory>>> = LazyLock::new(Default::default);
/// Chats kept in MEMORY; past this the oldest is forgotten.
const MEMORY_CHATS: usize = 64;

/// How much of an earlier exchange (bytes of JSON) the next answer may carry: little for a small
/// local model, a lot for a cloud one (about 100k tokens).
fn memory_budget(local: bool) -> usize {
    if local { 24_000 } else { 400_000 }
}

/// Makes `messages` fit in `budget`: old tool output first, then whole old exchanges. The list
/// always starts with a user line, so no tool result is left without its call.
fn trim_memory(messages: &mut Vec<Value>, budget: usize) {
    let size = |m: &[Value]| m.iter().map(|v| v.to_string().len()).sum::<usize>();
    for i in 0..messages.len() {
        if size(messages) <= budget {
            return;
        }
        if messages[i]["role"] == "tool" {
            messages[i]["content"] = json!("(Older output left out to save room. Run the step again if you need it.)");
        }
    }
    while size(messages) > budget && messages.len() > 1 {
        messages.remove(0);
        while messages.first().is_some_and(|m| !(m["role"] == "user" && m["content"].is_string())) {
            messages.remove(0);
        }
    }
}

/// The chat as the model saw it last time plus the messages written since, when the agent
/// answered in this chat before with the same model. None: build it from the chat lines.
async fn remembered(s: &AppState, chat_id: &str, role: &RoleAssignment) -> Option<Vec<Value>> {
    let (mut messages, until) = {
        let mem = MEMORY.lock().unwrap();
        let m = mem.get(chat_id).filter(|m| m.provider_id == role.provider_id && m.model_id == role.model_id)?;
        (m.messages.clone(), m.until.clone())
    };
    let newer: Vec<(String, String)> = sqlx::query_as("SELECT author, text FROM messages WHERE chat_id = ? AND at > ? ORDER BY at, rowid")
        .bind(chat_id)
        .bind(&until)
        .fetch_all(&s.db)
        .await
        .ok()?;
    if !newer.iter().any(|(a, _)| a == "user") {
        return None;
    }
    messages.extend(chat_lines(s, chat_id, role, newer).await);
    Some(messages)
}

/// CHAT-08: chat lines as the model reads them: attached documents as text and, for the newest
/// user line that has pictures, the pictures themselves when the model can see them. The newest
/// messages take their files first from one budget that follows the model's context window; the
/// rest of a long file is read with the attachment tools (CHAT-09).
async fn chat_lines(s: &AppState, chat_id: &str, role: &RoleAssignment, lines: Vec<(String, String)>) -> Vec<Value> {
    let (model, dir) = (role.model_id.as_str(), crate::chat_files::dir_for(&s.config.database));
    let mut plan = crate::attach_read::Plan::new(crate::context_windows::window_for(s, &role.provider_id, model), true);
    let mut pictures_left = true;
    let mut out = Vec::new();
    for (author, text) in lines.into_iter().rev() {
        let user = author == "user";
        let (text, files) = crate::chat_attach::split(&text);
        let text = text.split("\n\n:::sources\n").next().unwrap_or("").to_string();
        let (note, images) = if user && !files.is_empty() {
            let r = crate::chat_attach_web::for_model(&dir, chat_id, &files, model, pictures_left, &mut plan).await;
            if !r.1.is_empty() {
                pictures_left = false;
            }
            r
        } else {
            (String::new(), Vec::new())
        };
        let m = llm::ChatMessage { role: if user { "user".into() } else { "assistant".into() }, content: format!("{text}{note}"), images };
        out.push(m.wire(false));
    }
    out.reverse();
    out
}

/// One message as the next answer keeps it: without Anthropic's thinking blocks, whose signature
/// is bound to the request that made them (the next one rebuilds the system prompt, so Anthropic
/// refuses them as "bound to a different conversation"), and without pictures (too big to carry;
/// the note naming them stays).
fn for_memory(m: Value) -> Value {
    let mut m = crate::llm_anthropic::strip_blocks(std::slice::from_ref(&m)).remove(0);
    if let Some(parts) = m["content"].as_array() {
        let text: Vec<&str> = parts.iter().filter(|p| p["type"] == "text").filter_map(|p| p["text"].as_str()).collect();
        m["content"] = json!(text.join("\n\n"));
    }
    m
}

async fn remember(s: &AppState, chat_id: &str, role: &RoleAssignment, mut messages: Vec<Value>, answer: &str, local: bool) {
    messages.remove(0); // the system prompt is rebuilt every time
    let mut messages: Vec<Value> = messages.into_iter().map(for_memory).collect();
    messages.push(json!({ "role": "assistant", "content": answer }));
    trim_memory(&mut messages, memory_budget(local));
    let until: Option<String> = sqlx::query_scalar("SELECT MAX(at) FROM messages WHERE chat_id = ?").bind(chat_id).fetch_one(&s.db).await.ok().flatten();
    let Some(until) = until else { return };
    let mut mem = MEMORY.lock().unwrap();
    if mem.len() >= MEMORY_CHATS && !mem.contains_key(chat_id) {
        if let Some(oldest) = mem.iter().min_by(|a, b| a.1.until.cmp(&b.1.until)).map(|(k, _)| k.clone()) {
            mem.remove(&oldest);
        }
    }
    mem.insert(chat_id.to_string(), Memory { provider_id: role.provider_id.clone(), model_id: role.model_id.clone(), messages, until });
}

pub async fn run(s: AppState, user_id: String, chat_id: String, machine_id: String, machine_name: String, role: RoleAssignment) {
    STOPPED_CHATS.lock().unwrap().remove(&chat_id); // a stop from an earlier answer doesn't count
    let mut messages = vec![json!({ "role": "system", "content": system_prompt(&machine_name) })];
    // FACTS: the project's and the chat's facts (fresh each request, also when the exchange is remembered).
    if let Some((block, _)) = crate::facts::prompt_block(&s.db, &chat_id, time::OffsetDateTime::now_utc()).await {
        messages[0]["content"] = json!(format!("{}{block}", messages[0]["content"].as_str().unwrap_or("")));
    }
    if let Some(earlier) = remembered(&s, &chat_id, &role).await {
        messages.extend(earlier);
    } else {
        // CTX-01: the chat's summary first (when it has one), then the newest messages after it.
        let summary = crate::context::summary(&s.db, &chat_id).await.ok().flatten();
        if let Some(x) = &summary {
            messages[0]["content"] = json!(format!("{}{}", messages[0]["content"].as_str().unwrap_or(""), crate::context::prompt_block(x)));
        }
        let history = crate::context::after_summary(&s.db, &chat_id, summary.as_ref(), 20).await.unwrap_or_default();
        let lines = history.into_iter().map(|(_, author, text, _)| (author, text)).collect();
        messages.extend(chat_lines(&s, &chat_id, &role, lines).await);
        if let Some(d) = steplog::digest(&s.db, &user_id, &chat_id).await {
            messages.push(json!({ "role": "system", "content": d }));
        }
    }
    let effort: Option<String> = sqlx::query_scalar("SELECT effort FROM chats WHERE id = ?").bind(&chat_id).fetch_optional(&s.db).await.ok().flatten();
    let effort = effort.as_deref().and_then(crate::effort::Effort::parse).unwrap_or_default();
    let auto = auto_run_for(&s, &chat_id).await;
    let max_steps = max_steps_for(&s, &user_id).await;
    let local = s.provider(&role.provider_id).is_some_and(|p| p.local);
    let agent = Agent { s: s.clone(), user_id: user_id.clone(), chat_id: chat_id.clone(), machine_id, role, auto, max_steps, effort, folder: None, task_id: None, run_id: None, protect: None, tests_allowed: Vec::new() };
    let text = match agent.run_in(&mut messages).await {
        Ok(t) => t,
        Err(e) => e,
    };
    say(&s, &user_id, &chat_id, &text).await;
    remember(&s, &chat_id, &agent.role, messages, &text, local).await;
}

/// One agent working on one computer for one chat: the model calls the runner's
/// tools until it answers in text (or runs out of steps).
pub struct Agent {
    pub s: AppState,
    pub user_id: String,
    pub chat_id: String,
    pub machine_id: String,
    pub role: RoleAssignment,
    /// Steps already covered by a grant run without a card (tasks under standing
    /// grants); anything else still waits for the user's decision.
    pub auto: bool,
    pub max_steps: usize,
    /// EF-01: the level every model call of this agent runs at.
    pub effort: crate::effort::Effort,
    /// W2 tasks: every path and working folder must be inside this folder, so the check
    /// and the diff always see the folder that was worked in (never two mixed folders).
    pub folder: Option<String>,
    /// W2: the task this agent works for; when the user stops it, the agent stops too.
    pub task_id: Option<String>,
    /// W2: the run this agent works for (its model calls and steps are recorded with it).
    pub run_id: Option<String>,
    /// W2 tasks that may not change tests: Some(extra protected files); None = no protection.
    pub protect: Option<Vec<String>>,
    /// DOGFOOD: test files the task's description asks to change; those stay open (protect::named_in).
    pub tests_allowed: Vec<String>,
}

/// Why an edit_file job would break the file, if it would: reads the file through the
/// runner (no card; the runner still enforces the grants) and checks the whole-lines rule.
/// Without a read grant the check is skipped and the runner's exact match decides.
async fn edit_problem(s: &AppState, user_id: &str, machine_id: &str, job: &Value) -> Option<String> {
    let (path, old) = (job["path"].as_str()?, job["old"].as_str().unwrap_or(""));
    if old.trim().is_empty() {
        return tools::whole_lines("", old).err();
    }
    let id = crate::access::queue_job(&s.db, machine_id, user_id, &json!({ "tool": "read_file", "path": path }), None).await.ok()?;
    for _ in 0..120 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let row: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT state, result FROM machine_jobs WHERE id = ?").bind(&id).fetch_optional(&s.db).await.ok()?;
        match row {
            Some((st, out)) if st == "done" => return precheck(&out.unwrap_or_default(), old),
            Some((st, _)) if st == "failed" || st == "refused" => return None,
            _ => {}
        }
    }
    None
}

/// Job results are stored cut to 64 KiB (access/runner.rs); a read this long may be cut.
const STORED_RESULT_CAP: usize = 64 * 1024;

/// Checks `old` against the stored read of a file. A read at the cap may be cut, so `old`
/// could look "not found" when it is past the cut: skip the check and let the runner's own
/// whole-line check on the full file decide.
fn precheck(stored: &str, old: &str) -> Option<String> {
    if stored.len() >= STORED_RESULT_CAP - 4 {
        return None;
    }
    tools::whole_lines(stored, old).err()
}

/// The paths a runner job touches (path, or the working folder of a command).
fn job_paths(job: &Value) -> Vec<&str> {
    ["path", "cwd"].iter().filter_map(|k| job[*k].as_str()).collect()
}

/// Inside `folder` (the folder itself or below it), compared as clean absolute paths.
pub fn inside(folder: &str, path: &str) -> bool {
    let f = folder.trim_end_matches('/');
    path.starts_with('/') && !path.split('/').any(|c| c == "..") && (path == f || path.starts_with(&format!("{f}/")))
}

impl Agent {
    /// Runs the tool loop on `messages` (system prompt first). Ok(final text), or
    /// Err(text to tell the user) when the model failed or ran out of steps.
    pub async fn run(&self, mut messages: Vec<Value>) -> Result<String, String> {
        self.run_in(&mut messages).await
    }

    /// Like `run`, and `messages` keeps the whole exchange afterwards (for the next answer).
    pub async fn run_in(&self, messages: &mut Vec<Value>) -> Result<String, String> {
        let s = &self.s;
        let Some(p) = s.provider(&self.role.provider_id) else {
            return Err(format!("The provider {} is not configured.", self.role.provider_id));
        };
        let mut tools = tools::schema();
        // AGENT-SPAWN: a chat's agent may hand work on to a new chat or task (a task's may not).
        if self.task_id.is_none() && let Some(list) = tools.as_array_mut() {
            list.push(spawn::spec());
        }
        let mut spawned = 0;
        let mut sent_upto = 0usize;
        // HIST-01: the user's own past chats (answered here, read-only); FACTS: remember / forget
        // and the chat's step log.
        if let Some(list) = tools.as_array_mut() {
            list.extend(crate::chat_history::specs());
            list.extend(crate::facts::specs());
            list.push(steplog::spec());
        }
        // CHAT-09: the files attached in this chat are read in parts with these (only when there are any).
        if crate::attach_read::has_docs(&s.db, &crate::chat_files::dir_for(&s.config.database), &self.chat_id).await && let Some(list) = tools.as_array_mut() {
            list.extend(crate::attach_read::specs());
        }
        // KG-05: a task's worker also gets the tools of the MCP servers the user granted (and is denied the rest).
        let mut mcp_map = std::collections::HashMap::new();
        if self.task_id.is_some() {
            let (specs, map) = crate::mcp_grants::tools_for(s, &self.user_id).await;
            if let Some(list) = tools.as_array_mut() {
                list.extend(specs);
            }
            mcp_map = map;
        }
        let (p, model_id) = crate::effort::apply(&p, &self.role.model_id, self.effort);
        crate::context_windows::ensure(s, &p.id, &model_id).await;
        let budget = result_budget(p.local);
        let attach = crate::attach_read::Budget::new(crate::context_windows::window_for(s, &p.id, &model_id), true);
        // Answers in a row that hit the output limit before their tool call was complete.
        let mut cut_offs = 0;
        // DOGFOOD: an answer that only announced its next step ("Now I'll find the folder.") gets one nudge.
        let mut nudged = false;
        // Paths a read found missing during this answer (see the write_file guardrail).
        let mut missing = std::collections::HashSet::<String>::new();
        if self.task_id.as_deref().is_some_and(crate::taskrun::is_stopped) {
            return Err("stopped by you".into());
        }
        // AUTO-01: Automatic mode has no step cap; the user's Stop ends it at any step instead.
        for n in 0.. {
            if !self.auto && n >= self.max_steps {
                break;
            }
            if n > 0 && self.task_id.is_none() && chat_stopped(&self.chat_id) {
                return Err("stopped by you".into());
            }
            if n > 0 && self.task_id.as_deref().is_some_and(crate::taskrun::is_stopped) {
                return Err("stopped by you".into());
            }
            let started = std::time::Instant::now();
            let answer = llm::chat_with_tools_full(&s.http, &p, &model_id, &messages, &tools).await;
            let (msg, usage, err) = match answer {
                Ok((m, u)) => (m, u, None),
                Err(e) => {
                    let mut msg = format!("The model failed: {e:#}");
                    // CTX-02: a server that refused the prompt as too long names its limit: keep it for this model.
                    let chars: usize = messages.iter().map(|m| m.to_string().chars().count()).sum();
                    if let Some(note) = crate::context_windows::learn(s, &p.id, &model_id, &msg, chars).await {
                        msg = format!("{msg}\n\n{note}");
                    }
                    (Value::Null, Value::Null, Some(msg))
                }
            };
            // TASK-OVERVIEW: what this call got that the one before did not (the whole prompt the first
            // time), so a run's trace shows each call's input without storing the history again and again.
            let request = json!({ "messages": messages.len(), "new": &messages[sent_upto.min(messages.len())..] });
            sent_upto = messages.len();
            crate::api::log_call(s, &self.user_id, &self.chat_id, self.run_id.as_deref(), &self.role.role, &self.role,
                "A step on a computer.", &request, &msg, &usage, started.elapsed().as_millis(), err.as_deref(), self.effort).await;
            if let Some(e) = err {
                return Err(e);
            }
            let calls = msg["tool_calls"].as_array().cloned().unwrap_or_default();
            if calls.is_empty() && usage["finish_reason"] == "length" {
                // Cut off at max_tokens, usually in the middle of a big edit: that is not the end
                // of the work. Tell the model and let it go on in smaller pieces.
                cut_offs += 1;
                if cut_offs >= MAX_CUT_OFFS {
                    return Err(format!("The model's answer was cut off at its output limit {MAX_CUT_OFFS} times in a row, so nothing more was changed. Raise max_tokens for the provider {} or ask for a smaller step.", self.role.provider_id));
                }
                // Only its text goes back: the cut answer has no tool call to answer.
                if let Some(text) = msg["content"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
                    say(s, &self.user_id, &self.chat_id, text).await;
                    messages.push(json!({ "role": "assistant", "content": text }));
                }
                messages.push(json!({ "role": "user", "content": CUT_OFF_NOTE }));
                continue;
            }
            cut_offs = 0;
            if calls.is_empty() {
                let text = msg["content"].as_str().unwrap_or("").trim();
                if !nudged && announces_next_step(text) {
                    nudged = true;
                    say(s, &self.user_id, &self.chat_id, text).await;
                    messages.push(json!({ "role": "assistant", "content": text }));
                    messages.push(json!({ "role": "user", "content": NEXT_STEP_NOTE }));
                    continue;
                }
                return Ok(final_text(text, n));
            }
            // A short "what I'll do" sentence before the tools, when the model wrote one.
            if let Some(text) = msg["content"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
                say(s, &self.user_id, &self.chat_id, text).await;
            }
            messages.push(msg.clone());
            // IMAGE-01: pictures read in this round; tool results are text, so they follow as one user message.
            let mut pictures: Vec<(String, String)> = Vec::new();
            for call in &calls {
                // Stop pressed while the model was answering (or during an earlier call of this
                // answer): don't start another step.
                if self.task_id.is_none() && chat_stopped(&self.chat_id) {
                    return Err("stopped by you".into());
                }
                if self.task_id.as_deref().is_some_and(crate::taskrun::is_stopped) {
                    return Err("stopped by you".into());
                }
                let name = call["function"]["name"].as_str().unwrap_or("");
                let args: Value = call["function"]["arguments"]
                    .as_str()
                    .and_then(|a| serde_json::from_str(a).ok())
                    .unwrap_or_else(|| json!({}));
                let job = tools::to_job(name, &args);
                // Guardrail: an edit must replace whole lines of the file as it is now
                // (checked on the computer before the edit; a broken file is never written).
                let edit_err = match &job {
                    Some(j) if j["tool"] == "edit_file" => edit_problem(s, &self.user_id, &self.machine_id, j).await,
                    _ => None,
                };
                let result = match job {
                    // KG-05: an MCP tool, answered by its server under the user's grant.
                    _ if mcp_map.contains_key(name) => {
                        if let Some((server, tool)) = mcp_map.get(name) {
                            log_access(s, &self.machine_id, &self.user_id, "mcp", &format!("mcp:{server}"), tool).await;
                        }
                        crate::mcp_grants::call(s, &self.user_id, &mcp_map, name, &args).await
                    }
                    // Answered here, not by a computer.
                    None if name == "capabilities" => crate::capabilities::summary(s, &self.user_id).await,
                    None if name == spawn::NAME && self.task_id.is_none() => {
                        spawned += 1;
                        if spawned > spawn::MAX_PER_ANSWER {
                            format!("Refused: one answer starts at most {} chats or tasks.", spawn::MAX_PER_ANSWER)
                        } else {
                            match spawn::job(&args) {
                                Err(e) => format!("Refused, nothing made: {e}"),
                                Ok(job) => self.start_work(&job).await,
                            }
                        }
                    }
                    None if crate::chat_history::is_tool(name) => crate::chat_history::call(&s.db, &self.user_id, &self.chat_id, name, &args, budget).await,
                    None if crate::attach_read::is_tool(name) => crate::attach_read::call(s, &self.chat_id, name, &args, budget, attach).await,
                    None if crate::facts::is_tool(name) => crate::facts::call(s, &self.chat_id, name, &args).await,
                    None if name == steplog::READ => steplog::read(&s.db, &self.user_id, &self.chat_id, &args, budget).await,
                    None => "Unknown tool or wrong arguments.".to_string(),
                    // Guardrail: a file found missing earlier in this answer is not created
                    // behind the user's back.
                    Some(job) if self.folder.as_deref().is_some_and(|f| job_paths(&job).iter().any(|p| !inside(f, p))) => {
                        format!(
                            "Blocked: outside the task's folder {}. Work only there; if the work belongs in another folder, stop and say which.",
                            self.folder.as_deref().unwrap_or("")
                        )
                    }
                    Some(_) if edit_err.is_some() => format!("Refused, nothing changed: {}", edit_err.as_deref().unwrap_or("")),
                    Some(job)
                        if matches!(job["tool"].as_str(), Some("edit_file" | "write_file"))
                            && self.protect.as_ref().is_some_and(|x| crate::taskrun::protect::guarded(job["path"].as_str().unwrap_or(""), x, &self.tests_allowed)) =>
                    {
                        let path = job["path"].as_str().unwrap_or("");
                        refuse(s, &self.user_id, &self.chat_id, &self.machine_id, &job, self.run_id.as_deref(), &tests_refused(path)).await
                    }
                    // RUN-01: the shell is no way round it (cat >> test_x.py, sed -i, cp, rm…).
                    Some(job) if self.shell_writes_tests(&job).is_some() => {
                        let path = self.shell_writes_tests(&job).unwrap_or_default();
                        refuse(s, &self.user_id, &self.chat_id, &self.machine_id, &job, self.run_id.as_deref(), &tests_refused(&path)).await
                    }
                    // IMAGE-02: a picture needs a full path; the clear answer comes before anything is asked of the user.
                    Some(job) if matches!(job["tool"].as_str(), Some("read_image" | "send_image")) && !tools::picture_path_ok(job["path"].as_str().unwrap_or("")) => {
                        "Not run: give the picture's full path (starting with / or a drive letter, no `..`). Use list_dir to find it.".to_string()
                    }
                    // IMAGE-01: only a model that can see pictures, and a runner that knows the tool.
                    Some(job) if job["tool"] == "read_image" && !crate::chat_attach::sees_pictures(&model_id) => {
                        "Not run: this model can't look at pictures. Tell the user to pick a model that can (for example Claude or a vision model), or find another way to check the file.".to_string()
                    }
                    Some(job) if matches!(job["tool"].as_str(), Some("read_image" | "send_image")) && !tools::runner_reads_images(s.host.runner_version(&self.machine_id).as_deref()) => {
                        "Refused, nothing changed: the runner on this computer is older than 0.6.5 or has not reported its version, so it may not know the picture tools. Ask the user to update the runner (it reports its version once it runs).".to_string()
                    }
                    // WRITE-CHUNKS: an older runner ignores append/offset and would replace the file.
                    Some(job) if tools::writes_in_parts(&job) && !tools::runner_writes_in_parts(s.host.runner_version(&self.machine_id).as_deref()) => {
                        "Refused, nothing changed: the runner on this computer is too old to write a file in parts (it would replace the file). Write it whole, or ask the user to update the runner.".to_string()
                    }
                    Some(job) if job["tool"] == "write_file" && job["path"].as_str().is_some_and(|p| missing.contains(p)) => {
                        "Blocked: that file did not exist when you looked. Ask the user before creating a new file.".to_string()
                    }
                    Some(job) => {
                        let r = step(s, &self.user_id, &self.chat_id, &self.machine_id, &job, self.auto, self.run_id.as_deref(), budget).await;
                        if r.contains("no such file") && let Some(p) = job["path"].as_str() {
                            missing.insert(p.to_string());
                            // FACTS: a path fact pointing here may be out of date.
                            crate::facts::flag_missing_path(s, &self.chat_id, p).await;
                        }
                        r
                    }
                };
                let (result, picture) = if matches!(name, "read_image" | "send_image") { split_picture(&result) } else { (result, None) };
                let path = args["path"].as_str().unwrap_or("").to_string();
                let result = match (name, picture) {
                    // IMAGE-02: into the chat for the user, not to the model.
                    ("send_image", Some(url)) => picture::send(s, &self.user_id, &self.chat_id, &path, args["caption"].as_str().unwrap_or(""), &url).await,
                    (_, Some(url)) => {
                        pictures.push((path, url));
                        result
                    }
                    (_, None) => result,
                };
                let was_stopped = result.starts_with(STEP_STOPPED);
                messages.push(json!({ "role": "tool", "tool_call_id": call["id"], "content": result }));
                if was_stopped {
                    return Err("stopped by you".into());
                }
            }
            for (path, url) in pictures {
                let m = llm::ChatMessage { role: "user".into(), content: format!("The picture from read_image {path}:"), images: vec![url] };
                messages.push(m.wire(false));
            }
        }
        Err(format!("{STEP_LIMIT} {} steps. Tell me how to go on.", self.max_steps))
    }
}

impl Agent {
    /// AGENT-SPAWN: an approval card for `start_work` (none in Automatic mode), then the chat or task.
    async fn start_work(&self, job: &Value) -> String {
        let s = &self.s;
        let id = match ask(s, &self.user_id, &self.chat_id, &self.machine_id, job, &spawn::summary(job), self.auto, self.run_id.as_deref()).await {
            Ok((id, _)) => id,
            Err(e) => return e,
        };
        set_action(s, &id, "running", "").await;
        changed(s, &self.user_id);
        let (state, text) = match spawn::perform(s, &self.user_id, &self.chat_id, job).await {
            Ok(t) => ("done", t),
            Err(e) => ("failed", format!("failed: {e}")),
        };
        set_action(s, &id, state, &text).await;
        changed(s, &self.user_id);
        text
    }
}

/// Whether the computer's current grants (the server's mirror) already allow this
/// job, so it can run without asking.
pub async fn covered(s: &AppState, machine_id: &str, job: &Value) -> bool {
    let Some((target, rights)) = tools::grant_for(job) else { return job["tool"] == "system_info" };
    let rows: Vec<(String, String, Option<String>)> =
        sqlx::query_as("SELECT target, rights, expires FROM machine_grants WHERE machine_id = ?")
            .bind(machine_id)
            .fetch_all(&s.db)
            .await
            .unwrap_or_default();
    let now = util::now();
    rows.iter().any(|(t, r, expires)| {
        let target_ok = if target == "system" {
            t == "system"
        } else {
            t != "system" && std::path::Path::new(&target).starts_with(t)
        };
        let have: Vec<String> = serde_json::from_str(r).unwrap_or_default();
        target_ok && rights.iter().all(|x| have.iter().any(|h| h == x)) && expires.as_deref().is_none_or(|e| now.as_str() < e)
    })
}

async fn set_action(s: &AppState, id: &str, state: &str, result: &str) {
    let _ = sqlx::query("UPDATE pc_actions SET state = ?, result = ?, ended_at = CASE WHEN ? IN ('done', 'failed', 'refused', 'denied', 'stopped') THEN ? ELSE ended_at END WHERE id = ?")
        .bind(state)
        .bind(util::cut_marked(result, 4000))
        .bind(state)
        .bind(util::now())
        .bind(id)
        .execute(&s.db)
        .await;
}

/// At startup no step can still be running: the server that ran them is gone. Marks them failed.
pub async fn fail_orphaned_steps(s: &AppState) {
    let r = sqlx::query("UPDATE pc_actions SET state = 'failed', result = 'The server restarted while this step was running; its result is unknown.', ended_at = ? WHERE state = 'running'")
        .bind(util::now())
        .execute(&s.db)
        .await;
    if let Ok(done) = r {
        if done.rows_affected() > 0 {
            tracing::info!("marked {} step(s) left running as failed", done.rows_affected());
        }
    }
}

/// The approval card for a step: stored as pending (or running when `preapproved`), then the
/// user's decision is awaited. Ok((card id, "covered" | "approved" | "always")) to go on, else the
/// text for the model (declined, or no answer in time).
#[allow(clippy::too_many_arguments)]
async fn ask(s: &AppState, user_id: &str, chat_id: &str, machine_id: &str, job: &Value, summary: &str, preapproved: bool, run_id: Option<&str>) -> Result<(String, String), String> {
    let id = util::new_id();
    let stored = sqlx::query(
        "INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, created_at, run_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(chat_id)
    .bind(user_id)
    .bind(machine_id)
    .bind(job.to_string())
    .bind(summary)
    .bind(if preapproved { "running" } else { "pending" })
    .bind(util::now())
    .bind(run_id)
    .execute(&s.db)
    .await;
    if let Err(e) = stored {
        return Err(format!("Could not ask the user: {e}"));
    }
    changed(s, user_id);

    // A task run waiting for the user: say so in the project thread, with a link to the card's chat.
    if let (false, Some(run)) = (preapproved, run_id) {
        crate::thread::post_run(s, run, &format!("Waiting for your approval: {summary}. [Open it](#chat={chat_id})")).await;
    }
    // NOTIFY: tell the user where they are (push, mail) when they want that.
    if !preapproved {
        crate::notify::approval_needed(s.db.clone(), user_id.to_string(), chat_id.to_string(), summary.to_string());
    }

    let mut state = if preapproved { "covered".to_string() } else { "pending".to_string() };
    for _ in 0..if preapproved { 0 } else { WAIT_SECS } {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let row: Option<(String,)> = sqlx::query_as("SELECT state FROM pc_actions WHERE id = ?")
            .bind(&id)
            .fetch_optional(&s.db)
            .await
            .unwrap_or(None);
        state = row.map(|r| r.0).unwrap_or_else(|| "denied".into());
        if state != "pending" {
            break;
        }
    }
    match state.as_str() {
        "pending" => {
            set_action(s, &id, "denied", "No answer within 30 minutes.").await;
            changed(s, user_id);
            Err("The user did not answer; nothing was done.".into())
        }
        "denied" => Err("The user declined this step; nothing was done.".into()),
        _ => Ok((id, state)),
    }
}

/// One tool call: an approval card, then (if approved) a runner job. Returns the
/// text the model gets back.
async fn step(s: &AppState, user_id: &str, chat_id: &str, machine_id: &str, job: &Value, auto: bool, run_id: Option<&str>, budget: usize) -> String {

    // Under a standing grant (tasks): no card, the step runs right away.
    let preapproved = auto && covered(s, machine_id, job).await;
    let (id, state) = match ask(s, user_id, chat_id, machine_id, job, &tools::summary(job), preapproved, run_id).await {
        Ok(x) => x,
        Err(e) => return e,
    };
    let always = state == "always";

    // The computer only runs what is granted. Approve adds a grant for this one
    // step (10 minutes, removed again afterwards, the earlier grant on that target
    // restored); Always allow adds one for 24 hours.
    // Already covered by a standing grant: nothing to add or remove.
    let needed = if preapproved { None } else { tools::grant_for(job) };
    let mut restore: Option<Option<Value>> = None;
    if let Some((target, rights)) = &needed {
        set_action(s, &id, "granting", "").await;
        changed(s, user_id);
        let before = current_grant(s, machine_id, target).await;
        let expires = if always { util::in_hours(24) } else { in_minutes(10) };
        let grant = json!({ "tool": "add_grant", "grant": {
            "target": target, "rights": rights, "granted_by": user_name(s, user_id).await,
            "granted_at": util::now(), "expires": expires,
        } });
        let ok = match access::queue_job(&s.db, machine_id, user_id, &grant, None).await {
            Ok(job_id) => wait_job(s, &job_id).await.0 == "done",
            Err(_) => false,
        };
        if !ok {
            set_action(s, &id, "failed", "The computer did not confirm the grant.").await;
            changed(s, user_id);
            return "failed: the computer did not confirm the grant, so nothing was run.".into();
        }
        log_access(s, machine_id, user_id, "granted", target, if always { "always allow, 24 h" } else { "one step, 10 min" }).await;
        if !always {
            restore = Some(before);
        }
    }

    let grant_note = if preapproved {
        "standing grant"
    } else if needed.is_none() {
        "no grant needed"
    } else if always {
        "always allowed · 24 h"
    } else {
        "allowed once · 10 min · removed after"
    };
    let _ = sqlx::query("UPDATE pc_actions SET grant_note = ? WHERE id = ?").bind(grant_note).bind(&id).execute(&s.db).await;
    set_action(s, &id, "running", "").await;
    changed(s, user_id);
    let mut job_row: Option<String> = None;
    let (mut state, mut result) = match access::queue_job(&s.db, machine_id, user_id, &tools::runner_job(job), None).await {
        Ok(job_id) => {
            RUNNING_JOBS.lock().unwrap().insert(id.clone(), (machine_id.to_string(), job_id.clone()));
            let r = wait_job(s, &job_id).await;
            RUNNING_JOBS.lock().unwrap().remove(&id);
            job_row = Some(job_id);
            r
        }
        Err(e) => ("failed".to_string(), e.to_string()),
    };
    // Only a real Stop press counts, not the phrase appearing in the output.
    let stop_pressed = STOP_REQUESTED.lock().unwrap().remove(&id);
    if stop_pressed && result.contains("stopped by you") {
        state = "stopped".to_string();
    }

    // A one-step grant goes away again; an earlier grant on that target comes back.
    if let (Some(before), Some((target, _))) = (restore, &needed) {
        // Update the server's copy at once: the next step must not count on a grant that is
        // being removed (a run report showed a step refused while marked "standing grant").
        let _ = sqlx::query("DELETE FROM machine_grants WHERE machine_id = ? AND target = ?")
            .bind(machine_id)
            .bind(target)
            .execute(&s.db)
            .await;
        let _ = access::queue_job(&s.db, machine_id, user_id, &json!({ "tool": "revoke_grant", "target": target }), None).await;
        if let Some(g) = before {
            let _ = access::queue_job(&s.db, machine_id, user_id, &json!({ "tool": "add_grant", "grant": g }), None).await;
        }
        log_access(s, machine_id, user_id, "revoked", target, "one step done").await;
    }
    // IMAGE-01: the picture goes to the model as an image; the card and the stored result keep only a note.
    let mut picture = None;
    if state == "done" && result.starts_with("data:image/") {
        let url = std::mem::take(&mut result);
        result = format!("picture, {} KB", url.len() * 3 / 4 / 1024);
        if let Some(job_id) = &job_row {
            let _ = sqlx::query("UPDATE machine_jobs SET result = ? WHERE id = ?").bind(&result).bind(job_id).execute(&s.db).await;
        }
        picture = Some(url);
    }
    set_action(s, &id, &state, &result).await;
    changed(s, user_id);
    match picture {
        Some(url) => with_picture("The step ran. State: done.\nOutput:\nThe picture follows in the next message.", &url),
        None => model_result(&state, &result, budget),
    }
}

/// IMAGE-01: separates the text for the model from the picture (a data URL) in a step's answer.
const PICTURE_MARK: &str = "\n\u{1}picture\u{1}";

fn with_picture(text: &str, url: &str) -> String {
    format!("{text}{PICTURE_MARK}{url}")
}

/// (text for the model, picture). Only a `data:image/` url counts as a picture.
fn split_picture(answer: &str) -> (String, Option<String>) {
    match answer.split_once(PICTURE_MARK) {
        Some((text, url)) if url.starts_with("data:image/") => (text.to_string(), Some(url.to_string())),
        _ => (answer.to_string(), None),
    }
}

/// Refuse a job because it violates a rule (e.g., editing a test file).
/// Inserts a pc_actions row with state 'refused' and returns the reason.
impl Agent {
    /// RUN-01: the protected test file a shell job would write, if any.
    fn shell_writes_tests(&self, job: &Value) -> Option<String> {
        let protect = self.protect.as_ref()?;
        if job["tool"] != "shell" {
            return None;
        }
        crate::taskrun::protect::shell_writes(job["command"].as_str().unwrap_or(""))
            .into_iter()
            .find(|p| crate::taskrun::protect::guarded(p, protect, &self.tests_allowed))
    }
}

/// Why a step on a protected test file was refused, and what to do instead of working around it.
fn tests_refused(path: &str) -> String {
    format!(
        "{path} is a test file and this run may not change tests (\"Tests may change\" was off, and the task doesn't ask to change this file). \
         Don't work around it with another tool or the shell: fix the code so the tests pass, and if a test is missing, say in your answer which test to add."
    )
}

async fn refuse(s: &AppState, user_id: &str, chat_id: &str, machine_id: &str, job: &Value, run_id: Option<&str>, reason: &str) -> String {
    let id = util::new_id();
    let _ = sqlx::query(
        "INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, result, created_at, ended_at, run_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(chat_id)
    .bind(user_id)
    .bind(machine_id)
    .bind(job.to_string())
    .bind(tools::summary(job))
    .bind("refused")
    .bind(reason)
    .bind(util::now())
    .bind(util::now())
    .bind(run_id)
    .execute(&s.db)
    .await;
    changed(s, user_id);
    format!("Refused, nothing changed: {}", reason)
}

/// The answer shown when the model stops calling tools. An empty answer is never turned into
/// a bare "Done.": that claimed success without saying what happened.
/// What a model gets after an answer that only said what it would do next (once per answer).
const NEXT_STEP_NOTE: &str = "You said what you would do next but did not do it. Do it now with a tool; if you can't, say what you need from the user and stop.";

/// DOGFOOD: an answer whose last sentence announces a step instead of reporting ("Now I'll find the
/// project folder.", "Let me check the tests:"), so the work isn't left half done.
fn announces_next_step(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.len() > 600 || t.ends_with('?') {
        return false;
    }
    if t.ends_with(':') {
        return true;
    }
    let last = t.rsplit(['.', '!', '\n']).find(|s| !s.trim().is_empty()).unwrap_or(t).trim().to_lowercase();
    ["now i'll", "now i will", "let me", "i'll now", "i will now", "next, i'll", "next i'll", "first, i'll", "first i'll", "now let me"]
        .iter()
        .any(|p| last.starts_with(p))
}

fn final_text(text: &str, steps_before: usize) -> String {
    if !text.is_empty() {
        return text.to_string();
    }
    if steps_before == 0 {
        "The model sent an empty answer and ran no steps. Nothing was changed. Ask again or pick another model.".into()
    } else {
        format!("The model ran {steps_before} round(s) of steps but ended without a summary, so I can't say whether the whole request was finished. The step cards above show what ran; ask me what is left.")
    }
}

/// How much of a step's output the model gets: a small local model keeps the old 6,000 bytes,
/// a cloud model (1M context) sees much more, so it isn't sent back to read a file in slivers.
pub fn result_budget(local: bool) -> usize {
    if local { 6_000 } else { 40_000 }
}

/// What the model gets back from a step: the outcome first, then the output. Output over
/// `budget` keeps its start (a file's top, a command's first lines) and its end (the exit code,
/// the errors, where a read continues), with a note on what was left out between them.
fn model_result(state: &str, result: &str, budget: usize) -> String {
    let outcome = match state {
        "done" => "The step ran.",
        "refused" => "The computer refused the step (not granted).",
        "stopped" => STEP_STOPPED,
        _ => "The step failed.",
    };
    if result.len() <= budget {
        return format!("{outcome} State: {state}.\nOutput:\n{result}");
    }
    let head = cut(result, budget / 3);
    let mut tail_start = result.len() - (budget - head.len());
    while !result.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    let left_out = tail_start - head.len();
    // DOGFOOD: say which lines are missing, so the model reads just those (a re-read of the whole
    // file comes back cut the same way; a byte offset is hard for a model to aim with).
    let (from, to) = (head.matches('\n').count() + 1, result[..tail_start].matches('\n').count() + 1);
    format!(
        "{outcome} State: {state}.\nOutput (start and end):\n{head}\n[... lines {from} to {to} ({left_out} bytes) left out here; reading it all again gives the same cut. \
         To see them: grep -n for what you need, then read just those lines (sed -n '{from},{}p' FILE, a part at a time), or read_file with offset {} and length.]\n{}",
        (from + 80).min(to),
        head.len(),
        &result[tail_start..]
    )
}

fn in_minutes(m: i64) -> String {
    (time::OffsetDateTime::now_utc() + time::Duration::minutes(m))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

async fn user_name(s: &AppState, user_id: &str) -> String {
    sqlx::query_as::<_, (String,)>("SELECT name FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
        .unwrap_or_else(|| "kompanion".into())
}

/// The computer's current grant on `target`, as the runner's grant JSON, from the
/// server's mirror of its grants.json.
async fn current_grant(s: &AppState, machine_id: &str, target: &str) -> Option<Value> {
    let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT rights, granted_by, granted_at, expires FROM machine_grants WHERE machine_id = ? AND target = ?",
    )
    .bind(machine_id)
    .bind(target)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    row.map(|(rights, by, at, expires)| {
        json!({ "target": target, "rights": serde_json::from_str::<Value>(&rights).unwrap_or(json!([])),
                "granted_by": by, "granted_at": at, "expires": expires })
    })
}

async fn log_access(s: &AppState, machine_id: &str, user_id: &str, kind: &str, target: &str, detail: &str) {
    let _ = sqlx::query("INSERT INTO access_log (machine_id, user_id, at, kind, target, detail) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(machine_id)
        .bind(user_id)
        .bind(util::now())
        .bind(kind)
        .bind(target)
        .bind(detail)
        .execute(&s.db)
        .await;
}

/// Waits for a runner job to finish: (state, result).
async fn wait_job(s: &AppState, job_id: &str) -> (String, String) {
    for _ in 0..WAIT_SECS {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let row: Option<(String, Option<String>)> = sqlx::query_as("SELECT state, result FROM machine_jobs WHERE id = ?")
            .bind(job_id)
            .fetch_optional(&s.db)
            .await
            .unwrap_or(None);
        if let Some((st, res)) = row
            && matches!(st.as_str(), "done" | "failed" | "refused")
        {
            return (st, res.unwrap_or_default());
        }
    }
    ("failed".into(), "The computer did not answer within 30 minutes.".into())
}

#[derive(Deserialize)]
pub struct Decision {
    pub decision: String,
}

/// The user's answer to an approval card.
pub async fn decide(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<String>,
    Json(b): Json<Decision>,
) -> ApiResult<StatusCode> {
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT machine_id, tool, state FROM pc_actions WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&u.id)
            .fetch_optional(&s.db)
            .await?;
    let Some((machine_id, tool, state)) = row else { return Err(ApiError::NotFound) };
    if state != "pending" {
        return Err(ApiError::BadRequest("This step was already decided.".into()));
    }
    let new_state = match b.decision.as_str() {
        "deny" => "denied",
        "approve" => "approved",
        "always" => "always",
        _ => return Err(ApiError::BadRequest("Unknown decision.".into())),
    };
    sqlx::query("UPDATE pc_actions SET state = ?, decided_at = ? WHERE id = ? AND state = 'pending'")
        .bind(new_state)
        .bind(util::now())
        .bind(&id)
        .execute(&s.db)
        .await?;
    changed(&s, &u.id);
    Ok(StatusCode::NO_CONTENT)
}

/// The approval cards of a chat, oldest first.
pub async fn list(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(chat_id): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
    let rows: Vec<(String, String, String, String, Option<String>, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, machine_id, summary, state, result, created_at, tool, COALESCE(decided_at, created_at), ended_at FROM (
           SELECT * FROM pc_actions WHERE chat_id = ? AND user_id = ? ORDER BY created_at DESC LIMIT 50
         ) ORDER BY created_at",
    )
    .bind(&chat_id)
    .bind(&u.id)
    .fetch_all(&s.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, machine_id, summary, state, result, created_at, tool, started_at, ended_at)| {
                let job: Value = serde_json::from_str(&tool).unwrap_or(Value::Null);
                json!({ "id": id, "machineId": machine_id, "summary": summary, "state": state, "result": result,
                        "createdAt": created_at, "startedAt": started_at, "endedAt": ended_at, "needs": needs_text(&job), "tool": job })
            })
            .collect(),
    ))
}

/// What a step needs on the computer, for the card: "packages + root (asks for the password on the PC)".
fn needs_text(job: &Value) -> Option<String> {
    let (target, rights) = tools::grant_for(job)?;
    let mut text = rights.join(" + ");
    if target != "system" {
        text = format!("{text} on {target}");
    }
    if rights.contains(&"root") {
        text.push_str(" (asks for the password on the PC)");
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_travels_with_a_step_answer_and_only_as_a_data_url() {
        let a = with_picture("done", "data:image/png;base64,AAAA");
        assert_eq!(split_picture(&a), ("done".to_string(), Some("data:image/png;base64,AAAA".to_string())));
        assert_eq!(split_picture("plain text"), ("plain text".to_string(), None));
        let forged = format!("x{PICTURE_MARK}http://evil/x.png");
        assert_eq!(split_picture(&forged), (forged.clone(), None));
    }

    #[test]
    fn steps_per_request_are_clamped_and_zero_means_no_limit() {
        assert_eq!(clamp_steps(0), usize::MAX);
        assert_eq!(clamp_steps(1), 1);
        assert_eq!(clamp_steps(100), 100);
        assert_eq!(clamp_steps(500), 100);
        assert_eq!(clamp_steps(-5), 1);
    }

    #[test]
    fn a_task_folder_confines_paths() {
        assert!(inside("/home/k/demo", "/home/k/demo"));
        assert!(inside("/home/k/demo/", "/home/k/demo/names.py"));
        assert!(!inside("/home/k/demo", "/home/k/demo2/x"));
        assert!(!inside("/home/k/demo", "/home/k/demo/../etc"));
        assert!(!inside("/home/k/demo", "names.py"));
        assert_eq!(job_paths(&json!({ "tool": "shell", "cwd": "/a", "command": "ls" })), ["/a"]);
    }

    #[test]
    fn prompt_names_the_machine() {
        let p = system_prompt("soucouyant");
        assert!(p.contains("soucouyant") && p.contains("approves"));
    }

    #[test]
    fn an_empty_final_answer_is_never_a_bare_done() {
        assert_eq!(final_text("Installed git 2.45.", 3), "Installed git 2.45.");
        for (text, n) in [("", 0), ("", 4)] {
            let r = final_text(text, n);
            assert_ne!(r.trim(), "Done.");
            assert!(r.len() > 40, "{r}");
        }
        assert!(final_text("", 0).contains("Nothing was changed"));
        assert!(final_text("", 4).contains("can't say"));
    }

    #[test]
    fn model_result_says_the_outcome_first() {
        let r = model_result("done", "installed\nexit: 0", 6000);
        assert!(r.starts_with("The step ran. State: done."));
        assert!(r.ends_with("exit: 0"));
        assert!(model_result("refused", "not granted: packages", 6000).contains("refused"));
    }

    #[test]
    fn an_announced_step_is_not_an_answer() {
        assert!(announces_next_step("Now I'll find the project folder."));
        assert!(announces_next_step("Found the note. Let me check the README:"));
        assert!(announces_next_step("I'll now add the Release section."));
        assert!(!announces_next_step("Added a Release section to README.md that says --strict-cfg."));
        assert!(!announces_next_step("Which folder should I use?"));
        assert!(!announces_next_step("Done. I'll leave the tests as they are because they already pass."), "a report that mentions a choice is fine");
        assert!(!announces_next_step(""));
    }

    #[test]
    fn long_output_keeps_its_start_and_its_end() {
        let out = format!("TOP OF FILE\n{}\nexit: 0", "é".repeat(10_000));
        let r = model_result("done", &out, 6000);
        assert!(r.contains("TOP OF FILE"), "the start is not lost");
        assert!(r.ends_with("exit: 0"), "nor the end");
        assert!(r.contains("bytes) left out here"));
        // The note names the missing lines and how to read them.
        let lines: String = (1..=900).map(|i| format!("line {i}\n")).collect();
        let r = model_result("done", &lines, 6000);
        assert!(r.contains("[... lines 235 to 456 (1992 bytes) left out here"), "{r}");
        assert!(r.contains("sed -n '235,315p' FILE"));
        assert!(r.len() < 6300);
        assert!(!model_result("done", &out, result_budget(false)).contains("left out"), "a cloud model gets all of it");
    }

    #[test]
    fn precheck_skips_a_read_cut_at_the_stored_cap() {
        // A small file is checked as before.
        assert!(precheck("a\nb\n", "b\n").is_none());
        assert!(precheck("a\nb\n", "zzz\n").is_some());
        // A file read cut at the cap: `old` is past the cut, so it must not be called missing.
        let cut_read = "x".repeat(STORED_RESULT_CAP);
        assert!(precheck(&cut_read, "line past the cut\n").is_none());
    }

    #[test]
    fn memory_drops_old_output_first_then_old_exchanges() {
        let big = "x".repeat(5000);
        let mut m = vec![
            json!({ "role": "user", "content": "Read a" }),
            json!({ "role": "assistant", "content": "", "tool_calls": [{ "id": "1" }] }),
            json!({ "role": "tool", "tool_call_id": "1", "content": big }),
            json!({ "role": "assistant", "content": "Read it." }),
            json!({ "role": "user", "content": "Go on" }),
        ];
        trim_memory(&mut m, 2000);
        assert_eq!(m.len(), 5, "only the output went");
        assert!(m[2]["content"].as_str().unwrap().starts_with("(Older output"));
        trim_memory(&mut m, 120);
        assert_eq!(m[0]["content"], "Go on", "then whole exchanges, from a user line");
        let mut small = vec![json!({ "role": "user", "content": "Hi" })];
        trim_memory(&mut small, 10_000);
        assert_eq!(small.len(), 1);
    }

    #[test]
    fn memory_drops_thinking_and_pictures() {
        let step = json!({ "role": "assistant", "content": "Looking.", "tool_calls": [{ "id": "t" }],
            crate::llm_anthropic::ANTHROPIC_BLOCKS: [{ "type": "thinking", "thinking": "", "signature": "sig" }] });
        let kept = for_memory(step);
        assert!(kept.get(crate::llm_anthropic::ANTHROPIC_BLOCKS).is_none(), "no signed thinking replayed in a later request");
        assert_eq!(kept["tool_calls"][0]["id"], "t");
        let pic = json!({ "role": "user", "content": [{ "type": "image_url", "image_url": { "url": concat!("data:image/png;", "base64,AA") } }, { "type": "text", "text": "What is this?" }] });
        assert_eq!(for_memory(pic)["content"], "What is this?");
    }

    #[test]
    fn cuts_on_char_boundaries() {
        assert_eq!(cut("aé", 2), "a");
        assert_eq!(cut("abc", 10), "abc");
    }
}
