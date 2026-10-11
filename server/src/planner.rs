//! KG-02: the hierarchical planner. A goal becomes milestones and sized tasks with dependencies; the model's
//! answer is constrained by a JSON schema and checked here (sizing rules, valid dependencies). A proposal is
//! stored and enters the task tree only when the user applies it. A failed task gets a re-plan proposal.
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::error::{ApiError, ApiResult};
use crate::{tasktree, util};

/// Sizing rules: a task that touches more files or changes more lines than this must be split.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub max_files: usize,
    pub max_lines: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_files: 5, max_lines: 250 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanTask {
    pub key: String,
    pub title: String,
    pub goal: String,
    #[serde(default)]
    pub steps: Vec<String>,
    pub done_when: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub lines: u32,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Milestone {
    pub title: String,
    pub goal: String,
    pub tasks: Vec<PlanTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plan {
    pub title: String,
    pub goal: String,
    pub milestones: Vec<Milestone>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Issue {
    /// oversize, no_done_when, duplicate_key, unknown_dep, self_dep, dep_loop, empty_milestone
    pub kind: String,
    pub key: String,
    pub message: String,
}

fn issue(kind: &str, key: &str, message: String) -> Issue {
    Issue { kind: kind.into(), key: key.into(), message }
}

/// The JSON schema the model's answer must follow (response_format).
pub fn schema() -> Value {
    let task = json!({
        "type": "object",
        "properties": {
            "key": { "type": "string", "description": "short unique id, used in depends_on" },
            "title": { "type": "string" },
            "goal": { "type": "string" },
            "steps": { "type": "array", "items": { "type": "string" } },
            "done_when": { "type": "string", "description": "one fact that can be checked" },
            "files": { "type": "array", "items": { "type": "string" }, "description": "files the task touches" },
            "lines": { "type": "integer", "description": "estimated lines changed" },
            "depends_on": { "type": "array", "items": { "type": "string" }, "description": "keys of tasks that must be done first" }
        },
        "required": ["key", "title", "goal", "steps", "done_when", "files", "lines", "depends_on"]
    });
    json!({
        "type": "object",
        "properties": {
            "title": { "type": "string" },
            "goal": { "type": "string" },
            "milestones": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "goal": { "type": "string" },
                        "tasks": { "type": "array", "items": task }
                    },
                    "required": ["title", "goal", "tasks"]
                }
            }
        },
        "required": ["title", "goal", "milestones"]
    })
}

/// A plan from the model's answer (plain JSON, or JSON inside a fence or text).
pub fn parse(text: &str) -> Result<Plan, String> {
    let v = crate::taskrun::parse::json_in(text).ok_or("The answer holds no JSON.")?;
    serde_json::from_value(v).map_err(|e| format!("The answer is not a plan: {e}"))
}

fn tasks_of(plan: &Plan) -> impl Iterator<Item = &PlanTask> {
    plan.milestones.iter().flat_map(|m| m.tasks.iter())
}

fn is_oversize(t: &PlanTask, lim: &Limits) -> bool {
    let files: HashSet<&str> = t.files.iter().map(String::as_str).collect();
    files.len() > lim.max_files || t.lines > lim.max_lines
}

/// Keys of the tasks over the sizing limits, in plan order.
pub fn oversize(plan: &Plan, lim: &Limits) -> Vec<String> {
    tasks_of(plan).filter(|t| is_oversize(t, lim)).map(|t| t.key.clone()).collect()
}

/// Everything wrong with a plan: sizing, missing done-when, keys, dependencies.
pub fn issues(plan: &Plan, lim: &Limits) -> Vec<Issue> {
    let mut out = Vec::new();
    let mut keys: HashSet<&str> = HashSet::new();
    for m in &plan.milestones {
        if m.tasks.is_empty() {
            out.push(issue("empty_milestone", &m.title, format!("Milestone \"{}\" has no tasks.", m.title)));
        }
    }
    for t in tasks_of(plan) {
        if !keys.insert(t.key.as_str()) {
            out.push(issue("duplicate_key", &t.key, format!("The key \"{}\" is used twice.", t.key)));
        }
        if t.done_when.trim().is_empty() {
            out.push(issue("no_done_when", &t.key, format!("Task \"{}\" has no done-when.", t.key)));
        }
        if is_oversize(t, lim) {
            out.push(issue("oversize", &t.key, format!("Task \"{}\" is too big ({} lines, {} files).", t.key, t.lines, t.files.iter().collect::<HashSet<_>>().len())));
        }
    }
    let mut edges: Vec<(String, String)> = Vec::new();
    for t in tasks_of(plan) {
        for d in &t.depends_on {
            if *d == t.key {
                out.push(issue("self_dep", &t.key, format!("Task \"{}\" depends on itself.", t.key)));
            } else if !keys.contains(d.as_str()) {
                out.push(issue("unknown_dep", &t.key, format!("Task \"{}\" depends on \"{d}\", which is not in the plan.", t.key)));
            } else {
                edges.push((t.key.clone(), d.clone()));
            }
        }
    }
    let mut seen: HashSet<&str> = HashSet::new();
    for t in tasks_of(plan) {
        if seen.insert(t.key.as_str()) && tasktree::dep_cycle(&edges, &t.key, &t.depends_on.iter().filter(|d| keys.contains(d.as_str()) && **d != t.key).cloned().collect::<Vec<_>>()) {
            out.push(issue("dep_loop", &t.key, format!("Task \"{}\" is part of a dependency loop.", t.key)));
        }
    }
    out
}

/// What to tell the model when its plan broke a rule.
pub fn fix_request(found: &[Issue], lim: &Limits) -> String {
    let mut s = format!(
        "The plan breaks these rules. Answer with the complete corrected plan. A task may touch {} files and change {} lines at most: split bigger ones into smaller tasks with dependencies between them.\n",
        lim.max_files, lim.max_lines
    );
    for i in found {
        s.push_str(&format!("- {} ({}): {}\n", i.key, i.kind, i.message));
    }
    s
}

/// A task's description in the shape the task editor and runner expect.
pub fn description(t: &PlanTask) -> String {
    let steps = if t.steps.is_empty() {
        "1. Do the work described in the goal.".to_string()
    } else {
        t.steps.iter().enumerate().map(|(i, s)| format!("{}. {s}", i + 1)).collect::<Vec<_>>().join("\n")
    };
    let files = if t.files.is_empty() { String::new() } else { format!("\n\n**Files:** {}", t.files.join(", ")) };
    format!("**Goal:** {}\n\n**Steps**\n{steps}\n\n**Done when:** {}{files}", t.goal.trim(), t.done_when.trim())
}

/// Asks the model for a plan and asks again (up to 3 rounds in all) while it breaks a rule. `ask` gets the
/// chat messages and returns the model's answer. Returns the last plan with the issues still open (empty
/// when it is valid), or an error when the model failed or never produced a plan.
pub async fn generate<F, Fut>(mut ask: F, system: &str, user: &str, lim: &Limits) -> Result<(Plan, Vec<Issue>), String>
where
    F: FnMut(Vec<Value>) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let mut messages = vec![json!({ "role": "system", "content": system }), json!({ "role": "user", "content": user })];
    let mut last: Option<(Plan, Vec<Issue>)> = None;
    let mut last_err = String::from("The model made no plan.");
    for _ in 0..3 {
        let text = ask(messages.clone()).await?;
        messages.push(json!({ "role": "assistant", "content": text }));
        match parse(&text) {
            Ok(plan) => {
                let found = issues(&plan, lim);
                if found.is_empty() {
                    return Ok((plan, found));
                }
                messages.push(json!({ "role": "user", "content": fix_request(&found, lim) }));
                last = Some((plan, found));
            }
            Err(e) => {
                messages.push(json!({ "role": "user", "content": format!("{e} Answer only with the JSON plan.") }));
                last_err = e;
            }
        }
    }
    last.ok_or(last_err)
}

fn check(plan: &Plan) -> ApiResult<()> {
    match issues(plan, &Limits::default()).first() {
        Some(i) => Err(ApiError::BadRequest(i.message.clone())),
        None => Ok(()),
    }
}

async fn insert_task(
    tx: &mut sqlx::SqliteConnection,
    user_id: &str,
    project_id: &str,
    title: &str,
    description: &str,
    kind: &str,
    parent: Option<&str>,
) -> ApiResult<String> {
    let id = util::new_id();
    sqlx::query(
        "INSERT INTO tasks (id, project_id, title, description, state, position, updated_at, user_id, effort, sync_dirty, parent_id, kind)
         VALUES (?, ?, ?, ?, 'queued', (SELECT COALESCE(MAX(position), 0) + 1 FROM tasks WHERE project_id = ?), ?, ?, 'auto',
                 (SELECT kind = 'windshift' FROM projects WHERE id = ?), ?, ?)",
    )
    .bind(&id)
    .bind(project_id)
    .bind(title.chars().take(200).collect::<String>())
    .bind(description)
    .bind(project_id)
    .bind(util::now())
    .bind(user_id)
    .bind(project_id)
    .bind(parent)
    .bind(kind)
    .execute(&mut *tx)
    .await?;
    Ok(id)
}

async fn insert_deps(tx: &mut sqlx::SqliteConnection, ids: &HashMap<String, String>, plan: &Plan, inherited: &[String]) -> ApiResult<()> {
    for t in tasks_of(plan) {
        let me = &ids[&t.key];
        let mut deps: Vec<String> = t.depends_on.iter().map(|d| ids[d].clone()).collect();
        if t.depends_on.is_empty() {
            deps.extend(inherited.iter().cloned());
        }
        deps.sort();
        deps.dedup();
        for d in deps {
            sqlx::query("INSERT INTO task_deps (task_id, depends_on) VALUES (?, ?)").bind(me).bind(d).execute(&mut *tx).await?;
        }
    }
    Ok(())
}

/// Writes a plan into the tree: a goal (or the given existing goal), a milestone each, their tasks and
/// dependencies, all queued. Returns the new ids, the goal first (when one was made).
pub async fn apply(db: &SqlitePool, user_id: &str, project_id: &str, plan: &Plan, goal_id: Option<&str>) -> ApiResult<Vec<String>> {
    check(plan)?;
    let owns: Option<i64> = sqlx::query_scalar("SELECT 1 FROM projects WHERE id = ? AND user_id = ?").bind(project_id).bind(user_id).fetch_optional(db).await?;
    if owns.is_none() {
        return Err(ApiError::NotFound);
    }
    if let Some(g) = goal_id {
        tasktree::check_parent(db, user_id, None, project_id, "milestone", g).await?;
    }
    let mut tx = db.begin().await?;
    let mut created = Vec::new();
    let goal = match goal_id {
        Some(g) => g.to_string(),
        None => {
            let id = insert_task(&mut tx, user_id, project_id, &plan.title, &format!("**Goal:** {}\n\n**Steps**\n1. Finish the milestones below.\n\n**Done when:** All milestones are done.", plan.goal.trim()), "goal", None).await?;
            created.push(id.clone());
            id
        }
    };
    let mut ids: HashMap<String, String> = HashMap::new();
    for m in &plan.milestones {
        let mid = insert_task(&mut tx, user_id, project_id, &m.title, &format!("**Goal:** {}\n\n**Steps**\n1. Finish the tasks below.\n\n**Done when:** All its tasks are done.", m.goal.trim()), "milestone", Some(&goal)).await?;
        created.push(mid.clone());
        for t in &m.tasks {
            let tid = insert_task(&mut tx, user_id, project_id, &t.title, &description(t), "task", Some(&mid)).await?;
            ids.insert(t.key.clone(), tid.clone());
            created.push(tid);
        }
    }
    insert_deps(&mut tx, &ids, plan, &[]).await?;
    tx.commit().await?;
    Ok(created)
}

/// A re-plan: the new tasks become subtasks of the failed task; those without a dependency inside the
/// plan inherit the failed task's own dependencies.
pub async fn apply_replan(db: &SqlitePool, user_id: &str, failed_task: &str, plan: &Plan) -> ApiResult<Vec<String>> {
    check(plan)?;
    let (project_id, kind): (String, String) = sqlx::query_as("SELECT project_id, kind FROM tasks WHERE id = ? AND user_id = ?")
        .bind(failed_task)
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .ok_or(ApiError::NotFound)?;
    if !tasktree::parent_allowed("task", &kind) {
        return Err(ApiError::BadRequest(format!("A {kind} can't be re-planned into subtasks.")));
    }
    let inherited: Vec<String> = sqlx::query_scalar("SELECT depends_on FROM task_deps WHERE task_id = ?").bind(failed_task).fetch_all(db).await?;
    let mut tx = db.begin().await?;
    let mut created = Vec::new();
    let mut ids: HashMap<String, String> = HashMap::new();
    for t in tasks_of(plan) {
        let tid = insert_task(&mut tx, user_id, &project_id, &t.title, &description(t), "task", Some(failed_task)).await?;
        ids.insert(t.key.clone(), tid.clone());
        created.push(tid);
    }
    insert_deps(&mut tx, &ids, plan, &inherited).await?;
    tx.commit().await?;
    Ok(created)
}

/// Stores a proposal as pending and returns its id.
pub async fn store_proposal(db: &SqlitePool, user_id: &str, project_id: &str, task_id: Option<&str>, kind: &str, reason: &str, plan: &Plan) -> ApiResult<String> {
    let id = util::new_id();
    sqlx::query("INSERT INTO plan_proposals (id, user_id, project_id, task_id, kind, reason, plan, state, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?)")
        .bind(&id)
        .bind(user_id)
        .bind(project_id)
        .bind(task_id)
        .bind(kind)
        .bind(reason)
        .bind(serde_json::to_string(plan).map_err(|e| ApiError::Internal(e.into()))?)
        .bind(util::now())
        .execute(db)
        .await?;
    Ok(id)
}

/// The project's pending proposals, newest first, as JSON.
pub async fn pending(db: &SqlitePool, user_id: &str, project_id: &str) -> ApiResult<Vec<Value>> {
    let rows: Vec<(String, Option<String>, String, String, String, String)> = sqlx::query_as(
        "SELECT id, task_id, kind, reason, plan, created_at FROM plan_proposals
         WHERE user_id = ? AND project_id = ? AND state = 'pending' ORDER BY created_at DESC, id",
    )
    .bind(user_id)
    .bind(project_id)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, task_id, kind, reason, plan, at)| {
            let plan: Value = serde_json::from_str(&plan).unwrap_or(Value::Null);
            json!({ "id": id, "taskId": task_id, "kind": kind, "reason": reason, "plan": plan, "createdAt": at })
        })
        .collect())
}

/// Sets a pending proposal to applied or dismissed. NotFound for another user's or a closed one.
pub async fn close_proposal(db: &SqlitePool, user_id: &str, id: &str, state: &str) -> ApiResult<()> {
    let n = sqlx::query("UPDATE plan_proposals SET state = ? WHERE id = ? AND user_id = ? AND state = 'pending'")
        .bind(state)
        .bind(id)
        .bind(user_id)
        .execute(db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(())
}

#[cfg(test)]
#[path = "planner_tests.rs"]
mod tests;
