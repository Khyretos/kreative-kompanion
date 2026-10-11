//! KG-01: the task tree. A goal holds milestones, a milestone holds tasks, a task can hold subtasks;
//! dependencies say which tasks must be done first. Everything stays inside one project and one user.
use std::collections::{HashMap, HashSet};

use sqlx::SqlitePool;
use crate::error::{ApiError, ApiResult};

pub const KINDS: &[&str] = &["goal", "milestone", "task"];

pub fn check_kind(kind: &str) -> ApiResult<()> {
    if KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!("Unknown kind {kind}.")))
    }
}

pub fn parent_allowed(kind: &str, parent_kind: &str) -> bool {
    match (kind, parent_kind) {
        ("goal", _) => false,
        ("milestone", "goal") => true,
        ("task", "goal" | "milestone" | "task") => true,
        _ => false,
    }
}

pub fn dep_cycle(edges: &[(String, String)], task: &str, deps: &[String]) -> bool {
    // The task's own edges are replaced by `deps`; every other edge stays.
    let mut next: HashMap<&str, Vec<&str>> = HashMap::new();
    for (from, to) in edges.iter().filter(|(f, _)| f != task) {
        next.entry(from.as_str()).or_default().push(to.as_str());
    }
    let mut stack: Vec<&str> = deps.iter().map(String::as_str).collect();
    let mut visited = HashSet::new();
    while let Some(cur) = stack.pop() {
        if cur == task {
            return true;
        }
        if visited.insert(cur) {
            stack.extend(next.get(cur).into_iter().flatten().copied());
        }
    }
    false
}

pub(crate) async fn check_parent(
    db: &SqlitePool,
    user_id: &str,
    task_id: Option<&str>,
    project_id: &str,
    kind: &str,
    parent: &str,
) -> ApiResult<()> {
    let (parent_project, parent_kind): (String, String) =
        sqlx::query_as("SELECT project_id, kind FROM tasks WHERE id = ? AND user_id = ?")
            .bind(parent)
            .bind(user_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ApiError::BadRequest("The parent task does not exist.".into()))?;
    if parent_project != project_id {
        return Err(ApiError::BadRequest("The parent must be in the same project.".into()));
    }
    if !parent_allowed(kind, &parent_kind) {
        return Err(ApiError::BadRequest(format!("A {kind} can't be placed under a {parent_kind}.")));
    }
    if let Some(id) = task_id {
        let mut visited = HashSet::new();
        let mut current = Some(parent.to_string());
        while let Some(cur) = current {
            if cur == id {
                return Err(ApiError::BadRequest("A task can't be placed under itself or one of its own subtasks.".into()));
            }
            if !visited.insert(cur.clone()) {
                break;
            }
            current = sqlx::query_scalar::<_, Option<String>>("SELECT parent_id FROM tasks WHERE id = ? AND user_id = ?")
                .bind(&cur)
                .bind(user_id)
                .fetch_optional(db)
                .await?
                .flatten();
        }
    }
    Ok(())
}

pub(crate) async fn check_children(db: &SqlitePool, user_id: &str, task_id: &str, kind: &str) -> ApiResult<()> {
    let kinds: Vec<String> = sqlx::query_scalar("SELECT kind FROM tasks WHERE parent_id = ? AND user_id = ?")
        .bind(task_id)
        .bind(user_id)
        .fetch_all(db)
        .await?;
    match kinds.iter().find(|c| !parent_allowed(c, kind)) {
        Some(child) => Err(ApiError::BadRequest(format!("This task holds a {child}, so it can't be a {kind}."))),
        None => Ok(()),
    }
}

pub(crate) async fn set_deps(db: &SqlitePool, user_id: &str, task_id: &str, deps: &[String]) -> ApiResult<Vec<String>> {
    let mut list: Vec<String> = Vec::new();
    for d in deps {
        if !list.contains(d) {
            list.push(d.clone());
        }
    }
    let project: String = sqlx::query_scalar("SELECT project_id FROM tasks WHERE id = ? AND user_id = ?")
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .ok_or(ApiError::NotFound)?;
    for d in &list {
        let same: Option<i64> = sqlx::query_scalar("SELECT 1 FROM tasks WHERE id = ? AND user_id = ? AND project_id = ?")
            .bind(d)
            .bind(user_id)
            .bind(&project)
            .fetch_optional(db)
            .await?;
        if same.is_none() {
            return Err(ApiError::BadRequest("A dependency must be a task in the same project.".into()));
        }
    }
    let edges: Vec<(String, String)> = sqlx::query_as(
        "SELECT d.task_id, d.depends_on FROM task_deps d JOIN tasks t ON t.id = d.task_id WHERE t.project_id = ? AND t.user_id = ?",
    )
    .bind(&project)
    .bind(user_id)
    .fetch_all(db)
    .await?;
    if dep_cycle(&edges, task_id, &list) {
        return Err(ApiError::BadRequest("These dependencies would make a loop.".into()));
    }
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM task_deps WHERE task_id = ?").bind(task_id).execute(&mut *tx).await?;
    for d in &list {
        sqlx::query("INSERT INTO task_deps (task_id, depends_on) VALUES (?, ?)")
            .bind(task_id)
            .bind(d)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(list)
}

pub(crate) async fn deps_of(db: &SqlitePool, user_id: &str, project: Option<&str>) -> sqlx::Result<HashMap<String, Vec<String>>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT d.task_id, d.depends_on FROM task_deps d JOIN tasks t ON t.id = d.task_id
         WHERE t.user_id = ?2 AND (?1 IS NULL OR t.project_id = ?1) ORDER BY d.task_id, d.depends_on",
    )
    .bind(project)
    .bind(user_id)
    .fetch_all(db)
    .await?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (task, dep) in rows {
        map.entry(task).or_default().push(dep);
    }
    Ok(map)
}

pub(crate) async fn detach(db: &SqlitePool, user_id: &str, task_id: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE tasks SET parent_id = (SELECT parent_id FROM tasks WHERE id = ?1) WHERE parent_id = ?1 AND user_id = ?2")
        .bind(task_id)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "tasktree_tests.rs"]
mod tests;
