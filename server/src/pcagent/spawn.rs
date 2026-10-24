//! AGENT-SPAWN: `start_work`, the model's way to hand work on to a new chat (which starts
//! answering the prompt) or to a new task in the chat's project. Each call gets an approval card
//! like any other step (Automatic mode runs it without asking), and an answer starts a few at most.

use axum::{Extension, Json, extract::State};
use serde_json::{Value, json};

use crate::{AppState, auth::User, events::Event};

pub const NAME: &str = "start_work";
/// `start_work` calls one answer may make.
pub const MAX_PER_ANSWER: usize = 3;
const TITLE_MAX: usize = 120;
const PROMPT_MAX: usize = 8_000;

pub fn spec() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": NAME,
            "description": "Hand work on instead of doing it here: start a new chat that answers `prompt` on its own (no computer), or add a task to this chat's project with `prompt` as its description. The user approves it like any step. Use it for separate work the user asked for, never to split up what you are doing now. Answered by Kompanion itself, not a computer.",
            "parameters": {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["chat", "task"], "description": "chat: a new chat in this chat's project (or none); task: a task in this chat's project." },
                    "title": { "type": "string", "description": "A short title, up to 120 characters." },
                    "prompt": { "type": "string", "description": "chat: its first message, complete enough to work from without this chat; task: its description on several lines, with the goal, numbered steps and when it is done." }
                },
                "required": ["kind", "title", "prompt"]
            }
        }
    })
}

/// The job for the card, or what is wrong with the call (for the model).
pub fn job(args: &Value) -> Result<Value, String> {
    let kind = args["kind"].as_str().unwrap_or("");
    if kind != "chat" && kind != "task" {
        return Err("kind must be \"chat\" or \"task\".".into());
    }
    let title = args["title"].as_str().unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() || title.chars().count() > TITLE_MAX {
        return Err(format!("Give a title of 1 to {TITLE_MAX} characters."));
    }
    let prompt = args["prompt"].as_str().unwrap_or("").trim();
    if prompt.is_empty() || prompt.chars().count() > PROMPT_MAX {
        return Err(format!("Give a prompt of 1 to {PROMPT_MAX} characters."));
    }
    Ok(json!({ "tool": NAME, "kind": kind, "title": title, "prompt": prompt }))
}

/// One line for the card.
pub fn summary(job: &Value) -> String {
    let title = job["title"].as_str().unwrap_or("");
    if job["kind"] == "task" { format!("Add the task \"{title}\"") } else { format!("Start a new chat: \"{title}\"") }
}

/// Makes the chat (and starts its answer) or the task, for the owner of `chat_id`. The text says
/// what was made, with `chat=<id>` or `task=<id>` for the card's Open button.
pub async fn perform(s: &AppState, user_id: &str, chat_id: &str, job: &Value) -> Result<String, String> {
    let (kind, title, prompt) = (job["kind"].as_str().unwrap_or(""), job["title"].as_str().unwrap_or(""), job["prompt"].as_str().unwrap_or(""));
    let project: Option<String> = sqlx::query_scalar("SELECT project_id FROM chats WHERE id = ? AND user_id = ?")
        .bind(chat_id)
        .bind(user_id)
        .fetch_optional(&s.db)
        .await
        .map_err(|e| e.to_string())?
        .flatten();
    let user = User { id: user_id.to_string(), name: String::new() };
    if kind == "task" {
        let Some(project_id) = project else {
            return Err("This chat is not in a project, so there is no task list: start a chat instead.".into());
        };
        let body = serde_json::from_value(json!({ "projectId": project_id, "title": title, "description": prompt, "chatId": chat_id }))
            .map_err(|e| e.to_string())?;
        let (_, Json(task)) = crate::tasks::create(State(s.clone()), Extension(user), Json(body)).await.map_err(|e| e.to_string())?;
        s.bus.send(user_id, Event::Changed { what: "tasks", machine_id: None });
        return Ok(format!("Added the task \"{title}\" to this project (task={}). It waits in the task list until it is started.", task["id"].as_str().unwrap_or("")));
    }
    let body = serde_json::from_value(json!({ "title": title, "projectId": project })).map_err(|e| e.to_string())?;
    let (_, Json(chat)) = crate::api::create_chat(State(s.clone()), Extension(user), Json(body)).await.map_err(|e| e.to_string())?;
    let first = crate::api::insert_message(s, &chat.id, "user", prompt).await.map_err(|e| e.to_string())?;
    s.bus.send(user_id, Event::Changed { what: "chats", machine_id: None });
    s.bus.send(user_id, Event::Message { message: first });
    match crate::api::user_role(s, user_id, "orchestrator").await {
        Ok(Some(role)) => {
            tokio::spawn(crate::api::answer(s.clone(), user_id.to_string(), chat.id.clone(), role, true));
            Ok(format!("Started the chat \"{title}\" (chat={}); it is answering the prompt there now.", chat.id))
        }
        _ => Ok(format!("Started the chat \"{title}\" (chat={}) with the prompt; no model is set to answer it yet.", chat.id)),
    }
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
