//! OVR-01: the Overseer. A chat mode (the composer's Overseer chip) whose model sees all of the
//! user's projects and tasks: a fresh scan goes into every answer. It plans with the user and
//! proposes tasks (`TASK:` lines, created with one click), and may add context to a running task
//! (`INTERJECT:` lines): sent at once when the user allows it, otherwise offered as a button.
//! The model is the user's "overseer" role, so any configured provider works (local by default).

use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::{AppState, auth::User, error::{ApiError, ApiResult}, util};

/// A task the scan lists under a short reference (T1, T2, ...) the model can name.
#[derive(Debug, Clone)]
pub struct TaskRef {
    pub n: usize,
    pub id: String,
    pub title: String,
    pub state: String,
}

/// What the Overseer sees: the text for the prompt plus the projects and tasks it may refer to.
#[derive(Debug, Clone, Default)]
pub struct Scan {
    pub text: String,
    /// (id, name)
    pub projects: Vec<(String, String)>,
    pub tasks: Vec<TaskRef>,
}

/// A task the Overseer proposes; `project_id` is None for a project that doesn't exist yet.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposed {
    pub project: String,
    pub project_id: Option<String>,
    pub title: String,
    pub description: String,
}

/// Context for a running task; `sent` when it already went to the task.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Interjection {
    pub task_id: String,
    pub title: String,
    pub text: String,
    pub sent: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Proposal {
    pub tasks: Vec<Proposed>,
    pub interjections: Vec<Interjection>,
}

/// The user's Overseer settings: their own choice, else the server's `[overseer]` default.
pub async fn prefs(s: &AppState, user_id: &str) -> (String, bool) {
    let row: Option<(Option<String>, Option<bool>)> = sqlx::query_as("SELECT overseer_name, overseer_interject FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&s.db)
        .await
        .unwrap_or_default();
    let (name, interject) = row.unwrap_or_default();
    let name = name
        .filter(|n| !n.trim().is_empty())
        .or_else(|| s.config.overseer.name.clone().filter(|n| !n.trim().is_empty()))
        .unwrap_or_else(|| "Overseer".into());
    (name, interject.unwrap_or(s.config.overseer.interject))
}

const LABELS: [(&str, &str); 7] = [
    ("running", "running"),
    ("needs_input", "needs you"),
    ("waiting_resources", "waiting for a GPU or machine"),
    ("failed", "failed"),
    ("in_review", "in review"),
    ("queued", "queued"),
    ("done", "done"),
];

fn label(state: &str) -> &str {
    LABELS.iter().find(|(k, _)| *k == state).map(|(_, v)| *v).unwrap_or(state)
}

type TaskRow = (String, String, String, String, String, f64, String);

/// The scan: every project with its task counts, then the tasks that matter now (running, needing
/// the user, failed, in review, the next three queued) with their step; done tasks only as a count
/// and the last few titles. Bounded to about `max` characters.
pub async fn scan(db: &SqlitePool, user_id: &str, max: usize) -> Scan {
    let projects: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, name, description FROM projects WHERE user_id = ? ORDER BY updated_at DESC LIMIT 30",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let tasks: Vec<TaskRow> = sqlx::query_as(
        "SELECT id, project_id, title, state, step, progress, updated_at FROM tasks WHERE user_id = ?
         ORDER BY position, updated_at",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut out = Scan { projects: projects.iter().map(|(id, n, _)| (id.clone(), n.clone())).collect(), ..Default::default() };
    if projects.is_empty() {
        out.text = "No projects yet.".into();
        return out;
    }
    let mut text = String::new();
    for (pid, name, description) in &projects {
        let mine: Vec<&TaskRow> = tasks.iter().filter(|t| &t.1 == pid).collect();
        let counts: Vec<String> = LABELS
            .iter()
            .filter_map(|(k, l)| {
                let c = mine.iter().filter(|t| t.3 == *k).count();
                (c > 0).then(|| format!("{c} {l}"))
            })
            .collect();
        let mut block = format!("## {name}\n");
        let about: String = description.lines().next().unwrap_or("").chars().take(160).collect();
        if !about.trim().is_empty() {
            block.push_str(&format!("{}\n", about.trim()));
        }
        block.push_str(&format!("Tasks: {}\n", if counts.is_empty() { "none".into() } else { counts.join(", ") }));
        let mut queued = 0;
        let mut refs = Vec::new();
        for (id, _, title, state, step, progress, at) in &mine {
            let show = match state.as_str() {
                "done" => false,
                "queued" => {
                    queued += 1;
                    queued <= 3
                }
                _ => true,
            };
            if !show {
                continue;
            }
            let n = out.tasks.len() + refs.len() + 1;
            refs.push(TaskRef { n, id: id.clone(), title: title.clone(), state: state.clone() });
            let mut line = format!("- [T{n}] {title}: {}", label(state));
            if state == "running" && *progress > 0.0 {
                line.push_str(&format!(" ({}%)", (progress * 100.0).round() as i64));
            }
            if !step.trim().is_empty() && state != "queued" {
                line.push_str(&format!(", {}", step.trim().chars().take(140).collect::<String>()));
            }
            line.push_str(&format!(" (updated {})\n", at.get(..16).unwrap_or(at)));
            block.push_str(&line);
        }
        let done: Vec<&str> = mine.iter().rev().filter(|t| t.3 == "done").take(3).map(|t| t.2.as_str()).collect();
        if !done.is_empty() {
            block.push_str(&format!("Recently done: {}\n", done.join("; ")));
        }
        if text.len() + block.len() > max {
            text.push_str("(more projects not shown)\n");
            break;
        }
        out.tasks.extend(refs);
        text.push_str(&block);
        text.push('\n');
    }
    out.text = text.trim_end().to_string();
    out
}

/// The Overseer's system prompt.
pub fn prompt(name: &str, interject: bool, scan: &Scan, now: &str) -> String {
    let send = if interject {
        "It is added to that task before its next step, and the project thread shows it."
    } else {
        "The user decides whether to send it."
    };
    format!(
        "You are {name}, the Overseer of Kreative Kompanion. You see all of the user's projects and tasks: the scan \
below was taken just now ({now} UTC). Answer from the scan and never invent tasks, states or progress.\n\n\
When asked for a status: lead with what needs the user (needs you, failed), then what is running and its step, \
then what is next. Short lines, the task titles in bold, no task references like T3 in the text.\n\n\
When the user plans a project or a feature with you: ask at most one question when something vital is missing, \
then propose small, self-contained tasks, one per line, exactly like this (the user creates them with one click):\n\
TASK: <project name> | <task title> | <what to do, and when it is done>\n\
Use an existing project's exact name, or a short new name for a new project.\n\n\
When a running task clearly lacks context the user gave you, add one line:\n\
INTERJECT: T<n> | <the context in one or two sentences>\n\
{send} Only for running tasks, only with facts from the user or the scan, never to repeat its description.\n\n\
Be direct and concise. Never claim you ran, started, stopped or changed anything.\n\n\
# Scan\n\n{}",
        scan.text
    )
}

// OVR-01-PARSE: drafted by the local model.
pub fn parse(text: &str, scan: &Scan) -> Proposal {
    let mut p = Proposal::default();
    for raw in text.lines() {
        let line = raw.trim();
        let line = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")).unwrap_or(line).trim();
        let line = line.strip_prefix("**").unwrap_or(line);
        let Some((kind, rest)) = line.split_once(':') else { continue };
        let rest = rest.strip_prefix("**").unwrap_or(rest).trim();
        if kind.eq_ignore_ascii_case("TASK") && p.tasks.len() < 30 {
            let parts: Vec<&str> = rest.splitn(3, '|').map(|x| x.trim()).collect();
            if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
                continue;
            }
            let known = scan.projects.iter().find(|(_, n)| n.eq_ignore_ascii_case(parts[0]));
            p.tasks.push(Proposed {
                project: known.map_or(parts[0], |(_, n)| n.as_str()).to_string(),
                project_id: known.map(|(id, _)| id.clone()),
                title: parts[1].chars().take(200).collect(),
                description: parts.get(2).map_or(String::new(), |d| d.chars().take(4000).collect()),
            });
        } else if kind.eq_ignore_ascii_case("INTERJECT") && p.interjections.len() < 5 {
            let Some((r, t)) = rest.split_once('|') else { continue };
            let r = r.trim().trim_start_matches('[').trim_end_matches(']');
            let Ok(n) = r.trim_start_matches(['T', 't']).parse::<usize>() else { continue };
            let text: String = t.trim().chars().take(1000).collect();
            if let Some(task) = scan.tasks.iter().find(|x| x.n == n && x.state == "running")
                && !text.is_empty()
            {
                p.interjections.push(Interjection { task_id: task.id.clone(), title: task.title.clone(), text, sent: false });
            }
        }
    }
    p
}

/// The `:::overseer` line the app shows as task proposals and context buttons ("" when there is none).
pub fn block(p: &Proposal) -> String {
    if p.tasks.is_empty() && p.interjections.is_empty() {
        return String::new();
    }
    format!("\n\n:::overseer\n{}", serde_json::to_string(p).unwrap_or_default())
}

/// After an Overseer answer: sends the context lines when the user allows it, and returns the block.
pub async fn finish(s: &AppState, user_id: &str, name: &str, interject: bool, scan: &Scan, text: &str) -> String {
    let mut p = parse(text, scan);
    if interject {
        for i in p.interjections.iter_mut() {
            i.sent = send(s, user_id, name, &i.task_id, &i.title, &i.text).await;
        }
    }
    block(&p)
}

/// Adds context to a running task and says so in its project thread; false when it isn't running.
async fn send(s: &AppState, user_id: &str, name: &str, task_id: &str, title: &str, text: &str) -> bool {
    let row: Option<(String, String)> = sqlx::query_as("SELECT project_id, state FROM tasks WHERE id = ? AND user_id = ?")
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(&s.db)
        .await
        .unwrap_or_default();
    let Some((project_id, state)) = row else { return false };
    if state != "running" {
        return false;
    }
    crate::taskrun::interject(task_id, text);
    crate::thread::post(s, user_id, &project_id, &format!("**{name}** added context to **{title}**: {text}")).await;
    true
}

#[derive(Deserialize)]
pub struct InterjectBody {
    text: String,
}

/// POST /api/tasks/{id}/interject: the user sends an Overseer suggestion (or their own context) to a running task.
pub async fn interject(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<String>,
    Json(b): Json<InterjectBody>,
) -> ApiResult<StatusCode> {
    let text: String = b.text.trim().chars().take(1000).collect();
    if text.is_empty() {
        return Err(ApiError::BadRequest("Write the context to add.".into()));
    }
    let title: Option<String> = sqlx::query_scalar("SELECT title FROM tasks WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&u.id)
        .fetch_optional(&s.db)
        .await?;
    let Some(title) = title else { return Err(ApiError::NotFound) };
    let (name, _) = prefs(&s, &u.id).await;
    if !send(&s, &u.id, &name, &id, &title, &text).await {
        return Err(ApiError::BadRequest("That task isn't running any more.".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTasks {
    project: String,
    #[serde(default)]
    project_id: Option<String>,
    tasks: Vec<NewTaskBody>,
}

#[derive(Deserialize)]
pub struct NewTaskBody {
    title: String,
    #[serde(default)]
    description: String,
}

/// POST /api/overseer/tasks: creates proposed tasks, and their project when it is new.
pub async fn create_tasks(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Json(b): Json<NewTasks>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    if b.tasks.is_empty() || b.tasks.len() > 30 {
        return Err(ApiError::BadRequest("Propose 1 to 30 tasks.".into()));
    }
    let name: String = b.project.trim().chars().take(80).collect();
    let existing: Option<String> = match &b.project_id {
        Some(id) => sqlx::query_scalar("SELECT id FROM projects WHERE id = ? AND user_id = ?").bind(id).bind(&u.id).fetch_optional(&s.db).await?,
        None => sqlx::query_scalar("SELECT id FROM projects WHERE user_id = ? AND lower(name) = lower(?)").bind(&u.id).bind(&name).fetch_optional(&s.db).await?,
    };
    let project_id = match existing {
        Some(id) => id,
        None if b.project_id.is_some() => return Err(ApiError::NotFound),
        None => {
            if name.is_empty() {
                return Err(ApiError::BadRequest("Name the project.".into()));
            }
            let id = util::new_id();
            sqlx::query("INSERT INTO projects (id, name, updated_at, user_id) VALUES (?, ?, ?, ?)")
                .bind(&id)
                .bind(&name)
                .bind(util::now())
                .bind(&u.id)
                .execute(&s.db)
                .await?;
            s.bus.send(&u.id, crate::events::Event::Changed { what: "projects", machine_id: None });
            id
        }
    };
    let mut ids = Vec::new();
    for t in b.tasks {
        let body = serde_json::from_value(json!({ "project_id": project_id, "title": t.title, "description": t.description }))
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        let (_, Json(task)) = crate::tasks::create(State(s.clone()), Extension(u.clone()), Json(body)).await?;
        ids.push(task.get("id").cloned().unwrap_or(Value::Null));
    }
    Ok((StatusCode::CREATED, Json(json!({ "projectId": project_id, "taskIds": ids }))))
}

#[cfg(test)]
#[path = "overseer_tests.rs"]
mod tests;
