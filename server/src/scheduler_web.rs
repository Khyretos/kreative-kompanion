//! KG-03: the scheduler's API. GET/PUT /projects/{id}/scheduler read and set a project's settings (which
//! computers, their folders, the default check and budget), POST /projects/{id}/scheduler/tick runs one
//! round now, PUT /tasks/{id}/budget sets one task's own limits.
use std::collections::HashMap;

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState,
    auth::User,
    error::{ApiError, ApiResult},
    events::Event,
    scheduler::{self, Budget, Cand, Config},
    util,
};

async fn own_project(s: &AppState, user_id: &str, id: &str) -> ApiResult<String> {
    let row: Option<String> = sqlx::query_scalar("SELECT scheduler FROM projects WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(&s.db)
        .await?;
    row.ok_or(ApiError::NotFound)
}

/// The queued and running tasks of a project with what each waits on.
async fn queue(s: &AppState, user_id: &str, project_id: &str) -> ApiResult<Vec<Value>> {
    let states: HashMap<String, String> = sqlx::query_as("SELECT id, state FROM tasks WHERE project_id = ? AND user_id = ?")
        .bind(project_id)
        .bind(user_id)
        .fetch_all(&s.db)
        .await?
        .into_iter()
        .collect();
    let rows: Vec<(String, String, String, f64)> = sqlx::query_as(
        "SELECT id, title, state, position FROM tasks WHERE project_id = ? AND user_id = ? AND kind = 'task' AND state IN ('queued', 'running') ORDER BY position",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_all(&s.db)
    .await?;
    let mut out = Vec::new();
    for (id, title, state, position) in rows {
        let deps: Vec<(String,)> = sqlx::query_as("SELECT depends_on FROM task_deps WHERE task_id = ?").bind(&id).fetch_all(&s.db).await?;
        let c = Cand { id: id.clone(), position, deps: deps.into_iter().map(|d| d.0).collect(), machine: None, open_children: false };
        let waits = scheduler::waiting_on(&c, &states);
        let label = if state == "running" { "running" } else if waits.is_empty() { "ready" } else { "waiting" };
        out.push(json!({ "id": id, "title": title, "status": label, "waitsOn": waits }));
    }
    Ok(out)
}

/// GET /projects/{id}/scheduler
pub async fn get(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let text = own_project(&s, &u.id, &id).await?;
    Ok(Json(json!({ "config": Config::parse(&text), "defaults": Budget::DEFAULT, "queue": queue(&s, &u.id, &id).await? })))
}

/// PUT /projects/{id}/scheduler
pub async fn put(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Json(c): Json<Config>) -> ApiResult<Json<Value>> {
    own_project(&s, &u.id, &id).await?;
    c.validate().map_err(ApiError::BadRequest)?;
    for m in c.machines.keys() {
        let known: Option<i64> = sqlx::query_scalar("SELECT 1 FROM machines WHERE id = ? AND user_id = ?").bind(m).bind(&u.id).fetch_optional(&s.db).await?;
        if known.is_none() {
            return Err(ApiError::BadRequest(format!("Unknown computer {m}.")));
        }
    }
    // Compact JSON without spaces: the loop finds enabled projects with a LIKE on `"enabled":true`.
    let text = serde_json::to_string(&c).map_err(|e| ApiError::Internal(e.into()))?;
    sqlx::query("UPDATE projects SET scheduler = ?, updated_at = ? WHERE id = ? AND user_id = ?")
        .bind(&text)
        .bind(util::now())
        .bind(&id)
        .bind(&u.id)
        .execute(&s.db)
        .await?;
    s.bus.send(&u.id, Event::Changed { what: "projects", machine_id: None });
    Ok(Json(json!({ "config": c })))
}

/// POST /projects/{id}/scheduler/tick: one round now.
pub async fn tick(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    own_project(&s, &u.id, &id).await?;
    let started = scheduler::tick_project(&s, &u.id, &id).await;
    Ok(Json(json!({ "started": started.into_iter().map(|(task, machine)| json!({ "task": task, "machine": machine })).collect::<Vec<_>>() })))
}

#[derive(Deserialize)]
pub struct BudgetBody {
    minutes: Option<i64>,
    rounds: Option<i64>,
    tokens: Option<i64>,
}

/// PUT /tasks/{id}/budget: null clears a limit (the project's, then the default, applies).
pub async fn set_budget(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Json(b): Json<BudgetBody>) -> ApiResult<Json<Value>> {
    if [b.minutes, b.rounds, b.tokens].into_iter().flatten().any(|v| v < 1) {
        return Err(ApiError::BadRequest("Budgets must be 1 or more.".into()));
    }
    let done = sqlx::query("UPDATE tasks SET budget_minutes = ?, budget_rounds = ?, budget_tokens = ?, updated_at = ? WHERE id = ? AND user_id = ?")
        .bind(b.minutes)
        .bind(b.rounds)
        .bind(b.tokens)
        .bind(util::now())
        .bind(&id)
        .bind(&u.id)
        .execute(&s.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    s.bus.send(&u.id, Event::Changed { what: "tasks", machine_id: None });
    Ok(Json(json!({ "minutes": b.minutes, "rounds": b.rounds, "tokens": b.tokens })))
}
