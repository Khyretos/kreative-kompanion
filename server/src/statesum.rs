//! KG-07: the project state summary. Built from the task rows only (no model), stored on the project and
//! rewritten after every finished or merged task. The planner and the Overseer read it, so "what is left"
//! comes from one place that is never older than the last merge.
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::{
    AppState,
    auth::User,
    error::{ApiError, ApiResult},
};

/// Upper bound of the stored text, in bytes.
pub const LIMIT: usize = 2000;
const LEFT_SHOWN: usize = 8;
const LISTED: usize = 5;

#[derive(Debug, Clone)]
pub struct Row {
    pub id: String,
    pub parent_id: String,
    pub kind: String,
    pub title: String,
    pub state: String,
}

fn list(titles: &[&str], shown: usize) -> String {
    let mut out = titles.iter().take(shown).copied().collect::<Vec<_>>().join("; ");
    if titles.len() > shown {
        out.push_str(&format!("; and {} more", titles.len() - shown));
    }
    out
}

/// pure: the summary text. `merged` is (task title, PR link), newest first.
pub fn render(name: &str, at: &str, rows: &[Row], merged: &[(String, String)]) -> String {
    let tasks: Vec<&Row> = rows.iter().filter(|r| r.kind == "task").collect();
    let done = tasks.iter().filter(|r| r.state == "done").count();
    let mut out = format!("State of {name} ({at} UTC)\n");
    if tasks.is_empty() {
        out.push_str("Tasks: none yet\n");
    } else {
        out.push_str(&format!("Tasks: {done} of {} done\n", tasks.len()));
    }
    for g in rows.iter().filter(|r| r.kind == "goal") {
        let mine: Vec<&&Row> = tasks.iter().filter(|t| t.parent_id == g.id).collect();
        if !mine.is_empty() {
            out.push_str(&format!("Goal {}: {} of {} tasks done\n", g.title, mine.iter().filter(|t| t.state == "done").count(), mine.len()));
        }
    }
    let by = |states: &[&str]| -> Vec<&str> { tasks.iter().filter(|r| states.contains(&r.state.as_str())).map(|r| r.title.as_str()).collect() };
    let running = by(&["running"]);
    if !running.is_empty() {
        out.push_str(&format!("Running: {}\n", list(&running, LISTED)));
    }
    let attention: Vec<String> = tasks.iter().filter(|r| matches!(r.state.as_str(), "needs_input" | "failed" | "refused")).map(|r| format!("{} ({})", r.title, r.state)).collect();
    if !attention.is_empty() {
        let refs: Vec<&str> = attention.iter().map(String::as_str).collect();
        out.push_str(&format!("Needs attention: {}\n", list(&refs, LISTED)));
    }
    let review = by(&["in_review"]);
    if !review.is_empty() {
        out.push_str(&format!("In review: {}\n", list(&review, LISTED)));
    }
    let left: Vec<&str> = tasks.iter().filter(|r| r.state != "done").map(|r| r.title.as_str()).collect();
    if left.is_empty() {
        out.push_str("Left: nothing\n");
    } else {
        out.push_str(&format!("Left: {}\n", list(&left, LEFT_SHOWN)));
    }
    if let Some((title, pr)) = merged.first() {
        out.push_str(&format!("Last merged: {title} ({pr})\n"));
    }
    cut(out.trim_end())
}

/// Within LIMIT bytes, at a line end; a cut says so on its last line (RELIABILITY: never silent).
fn cut(text: &str) -> String {
    const MORE: &str = "\n(more lines left out)";
    if text.len() <= LIMIT {
        return text.to_string();
    }
    let mut end = LIMIT - MORE.len();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let head = &text[..end];
    format!("{}{MORE}", &head[..head.rfind('\n').unwrap_or(head.len())])
}

async fn build(db: &SqlitePool, project_id: &str, user_id: &str) -> sqlx::Result<Option<String>> {
    let Some(name): Option<String> = sqlx::query_scalar("SELECT name FROM projects WHERE id = ? AND user_id = ?").bind(project_id).bind(user_id).fetch_optional(db).await? else {
        return Ok(None);
    };
    let rows: Vec<(String, Option<String>, String, String, String)> =
        sqlx::query_as("SELECT id, parent_id, kind, title, state FROM tasks WHERE project_id = ? AND user_id = ? ORDER BY position, rowid").bind(project_id).bind(user_id).fetch_all(db).await?;
    let rows: Vec<Row> = rows.into_iter().map(|(id, parent, kind, title, state)| Row { id, parent_id: parent.unwrap_or_default(), kind, title, state }).collect();
    let events: Vec<(String, String)> = sqlx::query_as(
        "SELECT t.title, e.detail FROM task_events e JOIN tasks t ON t.id = e.task_id
         WHERE t.project_id = ? AND t.user_id = ? AND e.kind = 'forge' ORDER BY e.id DESC LIMIT 20",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_all(db)
    .await?;
    let merged: Vec<(String, String)> = events
        .into_iter()
        .filter_map(|(title, detail)| {
            let v: serde_json::Value = serde_json::from_str(&detail).ok()?;
            (v["state"]["to"] == "done").then(|| (title, v["pr"].as_str().unwrap_or("").to_string()))
        })
        .collect();
    let at = crate::util::now();
    Ok(Some(render(&name, at.get(..16).unwrap_or(&at), &rows, &merged)))
}

/// Rewrites and stores the summary; returns it. Call after a task finished or a PR merged.
pub async fn refresh(db: &SqlitePool, project_id: &str, user_id: &str) -> sqlx::Result<String> {
    let text = build(db, project_id, user_id).await?.unwrap_or_default();
    sqlx::query("UPDATE projects SET state_summary = ?, state_summary_at = ? WHERE id = ? AND user_id = ?")
        .bind(&text)
        .bind(crate::util::now())
        .bind(project_id)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(text)
}

/// The stored summary; an unwritten one is built (and stored) on first read. Empty for a foreign project.
pub async fn summary_of(db: &SqlitePool, project_id: &str, user_id: &str) -> String {
    let stored: Option<String> = sqlx::query_scalar("SELECT state_summary FROM projects WHERE id = ? AND user_id = ?").bind(project_id).bind(user_id).fetch_optional(db).await.ok().flatten();
    match stored {
        Some(s) if !s.is_empty() => s,
        Some(_) => refresh(db, project_id, user_id).await.unwrap_or_default(),
        None => String::new(),
    }
}

/// Fire-and-forget refresh for the project of a task that just finished.
pub fn refresh_for_task(db: SqlitePool, task_id: String, user_id: String) {
    tokio::spawn(async move {
        let project: Option<String> = sqlx::query_scalar("SELECT project_id FROM tasks WHERE id = ? AND user_id = ?").bind(&task_id).bind(&user_id).fetch_optional(&db).await.ok().flatten();
        if let Some(p) = project {
            let _ = refresh(&db, &p, &user_id).await;
        }
    });
}

/// GET /api/projects/{id}/state
pub async fn get(State(s): State<AppState>, Extension(u): Extension<User>, Path(project): Path<String>) -> ApiResult<Json<Value>> {
    let text = summary_of(&s.db, &project, &u.id).await;
    if text.is_empty() {
        return Err(ApiError::NotFound);
    }
    let at: Option<String> = sqlx::query_scalar("SELECT state_summary_at FROM projects WHERE id = ? AND user_id = ?").bind(&project).bind(&u.id).fetch_optional(&s.db).await?;
    Ok(Json(json!({ "summary": text, "at": at.unwrap_or_default() })))
}

#[cfg(test)]
#[path = "statesum_tests.rs"]
mod tests;
