//! CMD-01: "Explain command" on a step card. Generated on the first click with the user's explain
//! model (else the chat's model) at low effort, logged like any call, and kept per command hash so
//! opening it again costs nothing.

use axum::{Extension, Json, extract::{Path, State}};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{AppState, api::RoleAssignment, auth::User, error::{ApiError, ApiResult}, llm, util};

const REASON: &str = "explain command";
/// Characters of a command the model is shown at most (a heredoc can be long).
const COMMAND_MAX: usize = 6_000;

/// The user's settings, as stored in users.explain_prefs and sent to the app.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    pub on: bool,
    pub verbose: bool,
    /// The model that explains; none = the chat's own model.
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
}

impl Prefs {
    /// A model only counts as a pair; blank strings are none.
    pub fn normalised(mut self) -> Self {
        let blank = |x: &Option<String>| x.as_deref().is_none_or(|v| v.trim().is_empty());
        if blank(&self.provider_id) || blank(&self.model_id) {
            self.provider_id = None;
            self.model_id = None;
        }
        self
    }
}

pub async fn prefs(db: &sqlx::SqlitePool, user_id: &str) -> Prefs {
    let raw: Option<String> = sqlx::query_scalar("SELECT explain_prefs FROM users WHERE id = ?").bind(user_id).fetch_optional(db).await.ok().flatten();
    raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default()
}

pub async fn set_prefs(db: &sqlx::SqlitePool, user_id: &str, p: Prefs) -> sqlx::Result<()> {
    let p = p.normalised();
    sqlx::query("UPDATE users SET explain_prefs = ? WHERE id = ?").bind(json!(p).to_string()).bind(user_id).execute(db).await?;
    Ok(())
}

/// The command a step runs, as a person would type it (the card shows the same).
pub fn command_text(job: &Value) -> String {
    let s = |k: &str| job[k].as_str().unwrap_or("").to_string();
    match job["tool"].as_str().unwrap_or("") {
        "shell" => {
            let (cwd, cmd) = (s("cwd"), s("command"));
            if cwd.is_empty() { cmd } else { format!("cd {cwd} && {cmd}") }
        }
        "package" => {
            let names: Vec<&str> = job["names"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            format!("{} {} {}", s("manager"), s("action"), names.join(" ")).trim().to_string()
        }
        "service" => format!("systemctl {} {}", s("action"), s("unit")).trim().to_string(),
        "read_file" => format!("read {}", s("path")),
        "list_dir" => format!("ls {}", s("path")),
        "write_file" => format!("write {} ({} characters)", s("path"), job["content"].as_str().map_or(0, |c| c.chars().count())),
        "edit_file" => format!("edit {}", s("path")),
        "system_info" => format!("system_info {}", s("what")).trim().to_string(),
        other => format!("{other} {}", job).trim().to_string(),
    }
}

fn prompt(verbose: bool, lang: &str) -> String {
    let language = match lang {
        "es" => "Spanish",
        "nl" => "Dutch",
        _ => "English",
    };
    let how = if verbose {
        "Go through it part by part: each program, flag and argument, what it reads or changes, what can go wrong, and how to undo it if that applies. Use a short list."
    } else {
        "In two to four sentences: what it does and what it changes, if anything."
    };
    format!(
        "You explain a command that an assistant runs on the user's computer, for the user who reads it while it runs. {how} \
         Say plainly when it deletes or overwrites something, or talks to the network. Markdown, no heading, no preamble. Write in {language}."
    )
}

fn hash(verbose: bool, lang: &str, role: &RoleAssignment, command: &str) -> String {
    let mut h = Sha256::new();
    h.update(format!("{}\n{lang}\n{}/{}\n{command}", if verbose { "verbose" } else { "compact" }, role.provider_id, role.model_id));
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Ask {
    lang: String,
}

/// POST /api/actions/{id}/explain {lang}: the explanation of a step's command ({text, cached}).
pub async fn explain(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, body: Option<Json<Ask>>) -> ApiResult<Json<Value>> {
    let lang = body.map(|Json(a)| a.lang).filter(|l| matches!(l.as_str(), "en" | "es" | "nl")).unwrap_or_else(|| "en".into());
    let row: Option<(String, String)> = sqlx::query_as("SELECT chat_id, tool FROM pc_actions WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&u.id)
        .fetch_optional(&s.db)
        .await?;
    let Some((chat_id, tool)) = row else { return Err(ApiError::NotFound) };
    let p = prefs(&s.db, &u.id).await;
    if !p.on {
        return Err(ApiError::BadRequest("Explanations are off: turn on \"Explain running actions\" in Settings.".into()));
    }
    let role = match (p.provider_id, p.model_id) {
        (Some(provider_id), Some(model_id)) if s.provider(&provider_id).is_some() => {
            RoleAssignment { role: "explain".into(), provider_id, model_id, effort: "low".into() }
        }
        _ => {
            let wanted = if crate::overseer::chat_on(&s.db, &chat_id).await { "overseer" } else { "orchestrator" };
            crate::api::user_roles(&s, &u.id).await?.into_iter().find(|r| r.role == wanted)
                .ok_or_else(|| ApiError::BadRequest("No model is set for this chat yet. Pick one under Models and roles.".into()))?
        }
    };
    let job: Value = serde_json::from_str(&tool).unwrap_or(Value::Null);
    let command: String = command_text(&job).chars().take(COMMAND_MAX).collect();
    let key = hash(p.verbose, &lang, &role, &command);
    if let Some(text) = sqlx::query_scalar::<_, String>("SELECT text FROM command_explanations WHERE hash = ?").bind(&key).fetch_optional(&s.db).await? {
        return Ok(Json(json!({ "text": text, "cached": true, "model": role.model_id })));
    }
    let Some(provider) = s.provider(&role.provider_id) else {
        return Err(ApiError::BadRequest(format!("The provider {} is not configured.", role.provider_id)));
    };
    let (provider, model) = crate::effort::apply(&provider, &role.model_id, crate::effort::Effort::Low);
    let convo = vec![
        llm::ChatMessage::text("system", prompt(p.verbose, &lang)),
        llm::ChatMessage::text("user", format!("```sh\n{command}\n```")),
    ];
    let started = std::time::Instant::now();
    let (mut text, mut usage, mut error) = (String::new(), json!({}), None::<String>);
    match llm::stream_chat(&s.http, &provider, &model, &convo).await {
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
        error = Some("the model wrote nothing".into());
    }
    crate::api::log_call(&s, &u.id, &chat_id, None, &role.role, &role, REASON, &json!({ "messages": convo }), &json!(text), &usage,
        started.elapsed().as_millis(), error.as_deref(), crate::effort::Effort::Low).await;
    if let Some(e) = error {
        return Err(ApiError::BadRequest(format!("The explanation failed: {e}")));
    }
    sqlx::query("INSERT OR REPLACE INTO command_explanations (hash, text, model, created_at) VALUES (?, ?, ?, ?)")
        .bind(&key)
        .bind(&text)
        .bind(&role.model_id)
        .bind(util::now())
        .execute(&s.db)
        .await?;
    Ok(Json(json!({ "text": text, "cached": false, "model": role.model_id })))
}

#[cfg(test)]
#[path = "explain_tests.rs"]
mod tests;
