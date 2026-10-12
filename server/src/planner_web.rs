//! KG-02: the planner's API and its model call. POST /projects/{id}/plan makes a proposal from a goal,
//! POST /tasks/{id}/replan one for a task that failed or found new work, and a task that ends in
//! needs_input gets a re-plan proposal by itself. Proposals enter the tree only through apply.
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState, api,
    auth::User,
    error::{ApiError, ApiResult},
    events::Event,
    llm,
    planner::{self, Limits, Plan},
};

const PLAN_SYSTEM: &str = "You plan software and creative projects. Break the goal into 1 to 6 milestones, each with small, \
self-contained tasks. A task is one change someone would notice, touches at most {files} files and changes at most {lines} lines: \
split anything bigger. Give every task a unique short key, the files it touches, an estimate of lines changed, a done_when that \
can be checked (a test, a build, a visible fact) and depends_on with the keys of tasks that must be done first (empty when none). \
Order work so dependencies come first. Answer only with the JSON plan.";

const REPLAN_SYSTEM: &str = "A task of a project failed or needs more work than planned. Replace it with 1 to 6 smaller tasks that \
together reach its goal, using what is already done and what went wrong. Each touches at most {files} files and changes at most \
{lines} lines, has a unique short key, files, a lines estimate, a checkable done_when and depends_on (keys of tasks in this answer \
that come first). Put them in one milestone. Answer only with the JSON plan.";

fn system(template: &str, lim: &Limits) -> String {
    template.replace("{files}", &lim.max_files.to_string()).replace("{lines}", &lim.max_lines.to_string())
}

/// One planning run on the user's orchestrator model, schema-constrained, with the rule-fixing rounds.
async fn plan_with_model(s: &AppState, user_id: &str, system: &str, user: &str) -> ApiResult<(Plan, Vec<planner::Issue>)> {
    let role = api::user_role(s, user_id, "orchestrator").await?.ok_or_else(|| ApiError::BadRequest("Set the orchestrator model first.".into()))?;
    let provider = s.config.provider(&role.provider_id).ok_or_else(|| ApiError::BadRequest(format!("The provider {} is not configured.", role.provider_id)))?;
    let (p, model) = crate::effort::apply(provider, &role.model_id, crate::effort::Effort::default());
    let schema = planner::schema();
    let ask = |messages: Vec<Value>| {
        let (http, p, model, schema) = (s.http.clone(), p.clone(), model.clone(), schema.clone());
        async move {
            // A server without schema support answers 400: ask again without it (the parser and the checks still apply).
            let first = llm::chat_json(&http, &p, &model, &messages, &schema).await;
            let answer = match first {
                Ok(a) => a,
                Err(_) => llm::chat_with_tools_full(&http, &p, &model, &messages, &json!([])).await.map_err(|e| format!("{e:#}"))?,
            };
            Ok::<String, String>(answer.0["content"].as_str().unwrap_or("").to_string())
        }
    };
    planner::generate(ask, system, user, &Limits::default()).await.map_err(|e| ApiError::BadRequest(format!("Planning failed: {e}")))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalBody {
    goal: String,
    /// An existing goal task to plan under, instead of a new goal.
    #[serde(default)]
    goal_id: Option<String>,
}

/// POST /api/projects/{id}/plan: a goal in words becomes a pending proposal (nothing is created yet).
pub async fn propose(State(s): State<AppState>, Extension(u): Extension<User>, Path(project): Path<String>, Json(b): Json<GoalBody>) -> ApiResult<(StatusCode, Json<Value>)> {
    let goal = b.goal.trim();
    if goal.len() < 10 {
        return Err(ApiError::BadRequest("Describe the goal in a sentence or two.".into()));
    }
    let row: Option<(String, String)> = sqlx::query_as("SELECT name, description FROM projects WHERE id = ? AND user_id = ?").bind(&project).bind(&u.id).fetch_optional(&s.db).await?;
    let (name, description) = row.ok_or(ApiError::NotFound)?;
    let existing: Vec<(String, String, String)> = sqlx::query_as("SELECT kind, title, state FROM tasks WHERE project_id = ? AND user_id = ? ORDER BY position LIMIT 60").bind(&project).bind(&u.id).fetch_all(&s.db).await?;
    let list: String = existing.iter().map(|(k, t, st)| format!("- {k}: {t} ({st})")).collect::<Vec<_>>().join("\n");
    let state = crate::statesum::summary_of(&s.db, &project, &u.id).await;
    let dev = crate::devsettings_web::brief_of(&s.db, &project, &u.id).await;
    let user = format!("Project: {name}\n{description}\n\n{dev}{state}\n\nExisting tasks (do not repeat finished work):\n{}\n\nGoal: {goal}", if list.is_empty() { "none".into() } else { list });
    let (plan, left) = plan_with_model(&s, &u.id, &system(PLAN_SYSTEM, &Limits::default()), &user).await?;
    let id = planner::store_proposal(&s.db, &u.id, &project, b.goal_id.as_deref().filter(|g| !g.is_empty()), "plan", goal, &plan).await?;
    s.bus.send(&u.id, Event::Changed { what: "tasks", machine_id: None });
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "kind": "plan", "plan": plan, "issues": left }))))
}

/// GET /api/projects/{id}/plans: the pending proposals.
pub async fn list(State(s): State<AppState>, Extension(u): Extension<User>, Path(project): Path<String>) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(planner::pending(&s.db, &u.id, &project).await?))
}

async fn load(s: &AppState, user_id: &str, id: &str) -> ApiResult<(String, Option<String>, String, Plan)> {
    let row: Option<(String, Option<String>, String, String)> = sqlx::query_as("SELECT project_id, task_id, kind, plan FROM plan_proposals WHERE id = ? AND user_id = ? AND state = 'pending'")
        .bind(id)
        .bind(user_id)
        .fetch_optional(&s.db)
        .await?;
    let (project, task, kind, plan) = row.ok_or(ApiError::NotFound)?;
    let plan: Plan = serde_json::from_str(&plan).map_err(|e| ApiError::Internal(e.into()))?;
    Ok((project, task, kind, plan))
}

/// POST /api/plans/{id}/apply: writes the proposal into the task tree (queued tasks, nothing runs).
pub async fn apply(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let (project, task, kind, plan) = load(&s, &u.id, &id).await?;
    let ids = if kind == "replan" {
        planner::apply_replan(&s.db, &u.id, &task.ok_or(ApiError::NotFound)?, &plan).await?
    } else {
        planner::apply(&s.db, &u.id, &project, &plan, task.as_deref()).await?
    };
    planner::close_proposal(&s.db, &u.id, &id, "applied").await?;
    s.bus.send(&u.id, Event::Changed { what: "tasks", machine_id: None });
    Ok(Json(json!({ "created": ids })))
}

/// POST /api/plans/{id}/dismiss
pub async fn dismiss(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    planner::close_proposal(&s.db, &u.id, &id, "dismissed").await?;
    s.bus.send(&u.id, Event::Changed { what: "tasks", machine_id: None });
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ReplanBody {
    #[serde(default)]
    reason: String,
}

/// Builds a re-plan proposal for a task. `reason` says what failed or what new work appeared.
pub async fn replan_task(s: &AppState, user_id: &str, task_id: &str, reason: &str) -> ApiResult<(String, Plan, Vec<planner::Issue>)> {
    let row: Option<(String, String, String, String, String, String)> = sqlx::query_as("SELECT project_id, title, description, state, step, kind FROM tasks WHERE id = ? AND user_id = ?")
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(&s.db)
        .await?;
    let (project, title, description, state, step, kind) = row.ok_or(ApiError::NotFound)?;
    if !crate::tasktree::parent_allowed("task", &kind) {
        return Err(ApiError::BadRequest(format!("A {kind} can't be re-planned; re-plan one of its tasks.")));
    }
    let siblings: Vec<(String, String)> = sqlx::query_as("SELECT title, state FROM tasks WHERE project_id = ? AND user_id = ? AND kind = 'task' AND id <> ? ORDER BY position LIMIT 40").bind(&project).bind(user_id).bind(task_id).fetch_all(&s.db).await?;
    let others: String = siblings.iter().map(|(t, st)| format!("- {t} ({st})")).collect::<Vec<_>>().join("\n");
    let why = if reason.trim().is_empty() { "unknown" } else { reason.trim() };
    let overview = crate::statesum::summary_of(&s.db, &project, user_id).await;
    let user = format!("{overview}\n\nTask: {title}\nState: {state}, last step: {step}\nWhat went wrong or what is new: {why}\n\n{description}\n\nOther tasks of the project:\n{}", if others.is_empty() { "none".into() } else { others });
    let (plan, left) = plan_with_model(s, user_id, &system(REPLAN_SYSTEM, &Limits::default()), &user).await?;
    let id = planner::store_proposal(&s.db, user_id, &project, Some(task_id), "replan", why, &plan).await?;
    s.bus.send(user_id, Event::Changed { what: "tasks", machine_id: None });
    Ok((id, plan, left))
}

/// POST /api/tasks/{id}/replan
pub async fn replan(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Json(b): Json<ReplanBody>) -> ApiResult<(StatusCode, Json<Value>)> {
    let (pid, plan, left) = replan_task(&s, &u.id, &id, &b.reason).await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": pid, "kind": "replan", "plan": plan, "issues": left }))))
}

/// A task that ended in needs_input (not stopped by the user) gets a re-plan proposal, unless one is already pending.
pub fn spawn_replan(s: AppState, user_id: String, task_id: String, reason: String) {
    if reason.starts_with("stopped") {
        return;
    }
    tokio::spawn(async move {
        let open: Option<i64> = sqlx::query_scalar("SELECT 1 FROM plan_proposals WHERE task_id = ? AND kind = 'replan' AND state = 'pending'")
            .bind(&task_id)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten();
        if open.is_some() {
            return;
        }
        if let Err(e) = replan_task(&s, &user_id, &task_id, &reason).await {
            tracing::info!("no re-plan proposal for task {task_id}: {e:?}");
        }
    });
}
