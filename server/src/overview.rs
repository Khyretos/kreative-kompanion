//! PROJ-OVERVIEW: GET /api/projects/{id}/overview, one call with what a project's Overview shows: what it is,
//! where it lives, what it may reach, how far along it is, what happened lately and which models worked on it.
//! Every part comes from existing tables; a part that fails is `null` with its reason in `errors`, never a 500.
use std::collections::HashMap;
use std::time::Duration;

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::{
    AppState,
    auth::User,
    devsettings,
    error::{ApiError, ApiResult},
    scheduler, util,
};

/// The longest `about` text, in bytes.
pub const ABOUT_MAX: usize = 20_000;
/// How much of `about` the planner and a project chat read, in chars.
pub const ABOUT_IN_CONTEXT: usize = 1500;
/// How much of `about` the Overseer reads per project (it scans up to 30 projects), in chars.
pub const ABOUT_IN_SCAN: usize = 300;
/// The cached git remote and branch are read again after this many minutes.
const GIT_STALE_MINUTES: i64 = 60;
/// How long a git read on the repo computer may take.
const GIT_WAIT: Duration = Duration::from_secs(30);

/// One task row for the progress part.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TaskRow {
    pub id: String,
    pub parent_id: String,
    pub kind: String,
    pub title: String,
    pub state: String,
}

/// The project columns the overview reads.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub about: String,
    pub ptype: String,
    pub kind: String,
    pub source: Option<String>,
    pub links: String,
    pub created_at: Option<String>,
    pub updated_at: String,
    pub archived_at: Option<String>,
    pub repo_folder: Option<String>,
    pub repo_machine_id: Option<String>,
    pub repo_git: String,
    pub dev_settings: String,
    pub scheduler: String,
    pub overseer: bool,
    pub owner: String,
}

/// pure: true when a computer grant on `target` reaches `folder`.
pub fn covers(target: &str, folder: &str) -> bool {
    // "/" trimmed of its slash would be empty: keep the root as "/".
    let trim = |p: &str| -> String {
        let t = p.trim_end_matches('/');
        if t.is_empty() && p.starts_with('/') { "/".into() } else { t.into() }
    };
    let (target, folder) = (trim(target), trim(folder));
    if folder.is_empty() || !target.starts_with('/') {
        return false;
    }
    target == "/" || target == folder || folder.starts_with(&format!("{target}/"))
}

/// pure: the project text a prompt gets: the description, then at most `max` chars of `about`.
pub fn with_about(description: &str, about: &str, max: usize) -> String {
    let d = description.trim();
    let a = about.trim();
    if a.is_empty() {
        return d.to_string();
    }
    let mut cut = a.chars().take(max).collect::<String>();
    if a.chars().count() > max {
        cut.push('…');
    }
    if d.is_empty() {
        cut
    } else {
        format!("{d}\n\n{cut}")
    }
}

/// pure: one activity line. `src` is "task" (a task event), "run" (a task run) or "chat".
pub fn activity_what(src: &str, kind: &str, title: &str, detail: &str) -> String {
    match src {
        "task" => match kind {
            "created" => format!("Task added: {}", title),
            "changed" => format!("Task changed: {}", title),
            "forge" => {
                let v: serde_json::Value = detail.parse().unwrap_or_default();
                if let Some(to) = v.get("state").and_then(|s| s.get("to")).and_then(|t| t.as_str()) {
                    if to == "done" {
                        format!("PR merged: {}", title)
                    } else {
                        format!("PR update: {}", title)
                    }
                } else {
                    format!("PR update: {}", title)
                }
            }
            _ => format!("{}: {}", kind.replace('_', " "), title),
        },
        "run" => format!("Run {}: {}", kind, title),
        _ => format!("Chat: {}", title),
    }
}

/// pure: progress counts from the task rows; `deps` holds (task_id, depends_on) pairs.
pub fn progress(rows: &[TaskRow], deps: &[(String, String)]) -> Value {
    let state: HashMap<&str, &str> = rows.iter().map(|r| (r.id.as_str(), r.state.as_str())).collect();
    let tasks: Vec<&TaskRow> = rows.iter().filter(|r| r.kind == "task").collect();
    let count = |states: &[&str]| tasks.iter().filter(|t| states.contains(&t.state.as_str())).count();
    let goals: Vec<Value> = rows
        .iter()
        .filter(|g| g.kind == "goal")
        .filter_map(|g| {
            let mine: Vec<&&TaskRow> = tasks.iter().filter(|t| t.parent_id == g.id).collect();
            let done = mine.iter().filter(|t| t.state == "done").count();
            (!mine.is_empty()).then(|| json!({ "title": g.title, "done": done, "total": mine.len() }))
        })
        .collect();
    // A dependency on a task that is not in the rows counts as done.
    let ready = |id: &str| deps.iter().filter(|(t, _)| t == id).all(|(_, on)| state.get(on.as_str()).is_none_or(|s| *s == "done"));
    let queued: Vec<&&TaskRow> = tasks.iter().filter(|t| t.state == "queued").collect();
    let (first, later): (Vec<&&TaskRow>, Vec<&&TaskRow>) = queued.into_iter().partition(|t| ready(&t.id));
    let next: Vec<Value> = first
        .iter()
        .map(|t| (t, true))
        .chain(later.iter().map(|t| (t, false)))
        .take(3)
        .map(|(t, r)| json!({ "id": t.id, "title": t.title, "ready": r }))
        .collect();
    json!({
        "done": count(&["done"]), "total": tasks.len(), "running": count(&["running"]),
        "needsYou": count(&["needs_input", "failed", "refused"]), "inReview": count(&["in_review"]),
        "goals": goals, "next": next,
    })
}

/// pure: the remote and branch from the output of `git remote get-url origin; git branch --show-current`.
pub fn parse_git(output: &str) -> Option<(String, String)> {
    let lines: Vec<&str> = output.lines().collect();
    let exit_idx = lines.iter().rposition(|l| l.trim_start().starts_with("exit:"));
    
    if let Some(idx) = exit_idx {
        let exit_line = lines[idx].trim();
        let num_part = exit_line.strip_prefix("exit:").map(|s| s.trim());
        if num_part != Some("0") {
            return None;
        }
    } else {
        return None;
    }
    
    let body_lines: Vec<String> = lines.iter()
        .take(if let Some(idx) = exit_idx { idx } else { lines.len() })
        .filter_map(|l| {
            let trimmed = l.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect();
    
    if body_lines.len() < 2 {
        return None;
    }
    
    let remote = body_lines[0].clone();
    let branch = body_lines[1].clone();
    
    // Drop "user:token@" between "://" and the host (only when the '@' comes before the next '/').
    let clean_remote = match remote.find("://") {
        Some(pos) => {
            let after = &remote[pos + 3..];
            match after.find('@') {
                Some(at) if after.find('/').is_none_or(|slash| at < slash) => format!("{}{}", &remote[..pos + 3], &after[at + 1..]),
                _ => remote.clone(),
            }
        }
        None => remote.clone(),
    };

    Some((clean_remote, branch))
}

/// Reads the repo's git remote and branch on its computer (a shell job under the computer's grants) and
/// caches them in `projects.repo_git`. Runs in the background; a failure leaves the cache as it was.
pub async fn refresh_git(s: AppState, user_id: String, project_id: String, machine_id: String, folder: String) {
    s.host.watch(&user_id);
    let job = match crate::access::queue_job(&s.db, &machine_id, &user_id, &json!({ "tool": "shell", "cwd": folder, "command": "git remote get-url origin; git branch --show-current" }), None).await {
        Ok(j) => j,
        Err(_) => return,
    };

    let deadline = tokio::time::Instant::now() + GIT_WAIT;
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let row: Option<(String, Option<String>)> = sqlx::query_as("SELECT state, result FROM machine_jobs WHERE id = ?")
            .bind(&job)
            .fetch_optional(&s.db)
            .await.ok().flatten();
        if let Some((state, result)) = row {
            if state == "done" {
                if let Some((remote, branch)) = parse_git(&result.unwrap_or_default()) {
                    let _ = sqlx::query("UPDATE projects SET repo_git = ? WHERE id = ? AND user_id = ?")
                        .bind(json!({ "remote": remote, "branch": branch, "at": util::now() }).to_string())
                        .bind(&project_id)
                        .bind(&user_id)
                        .execute(&s.db)
                        .await;
                }
                return;
            } else if state == "failed" || state == "refused" {
                return;
            }
        }
    }
    // Time is up: a job the computer never picked up is dropped.
    let _ = sqlx::query("DELETE FROM machine_jobs WHERE id = ? AND state = 'queued'").bind(&job).execute(&s.db).await;
}

/// The project row with its owner's name; None when it is not this user's.
pub async fn load(db: &SqlitePool, user_id: &str, project_id: &str) -> sqlx::Result<Option<ProjectRow>> {
    sqlx::query_as::<_, ProjectRow>(
        "SELECT p.id, p.name, p.description, p.about, p.ptype, p.kind, p.source, p.links, COALESCE(p.created_at, p.updated_at) AS created_at, p.updated_at, p.archived_at, p.repo_folder, p.repo_machine_id, p.repo_git, p.dev_settings, p.scheduler, p.overseer, COALESCE(u.name, '') AS owner FROM projects p LEFT JOIN users u ON u.id = p.user_id WHERE p.id = ? AND p.user_id = ?",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(db)
    .await
}

/// Where it lives: the repo computer, the folder and the cached git remote and branch.
async fn where_part(s: &AppState, user_id: &str, p: &ProjectRow) -> Result<Value, String> {
    let none = || json!({ "machine": null, "folder": p.repo_folder, "git": null, "gitState": "none" });
    let Some(machine_id) = p.repo_machine_id.as_deref() else { return Ok(none()) };
    let row: Option<(String, String, bool)> = sqlx::query_as(
        "SELECT id, name, last_seen IS NOT NULL AND (julianday('now') - julianday(last_seen)) * 86400 <= 90 FROM machines WHERE id = ? AND user_id = ?",
    )
    .bind(machine_id)
    .bind(user_id)
    .fetch_optional(&s.db)
    .await
    .map_err(|e| e.to_string())?;
    let Some((id, name, online)) = row else { return Ok(none()) };
    let cache: Value = serde_json::from_str(&p.repo_git).unwrap_or(Value::Null);
    let git = if cache.is_object() { cache } else { Value::Null };
    let stale = git["at"].as_str().is_none_or(|at| at < util::minutes_ago(GIT_STALE_MINUTES).as_str());
    let state = match (&p.repo_folder, online && stale) {
        (Some(folder), true) => {
            tokio::spawn(refresh_git(s.clone(), user_id.to_string(), p.id.clone(), id.clone(), folder.clone()));
            "reading"
        }
        _ if !git.is_null() => "known",
        _ => "unknown",
    };
    Ok(json!({
        "machine": { "id": id, "name": name, "online": online, "os": s.host.os(&id) },
        "folder": p.repo_folder, "git": git, "gitState": state,
    }))
}

/// What it can reach: computer grants covering the repo folder, active MCP grants, knowledge, assets, commands.
async fn access_part(s: &AppState, user_id: &str, p: &ProjectRow) -> Result<Value, String> {
    let err = |e: sqlx::Error| e.to_string();
    let mut grants = Vec::new();
    if let (Some(machine_id), Some(folder)) = (&p.repo_machine_id, &p.repo_folder) {
        let rows: Vec<(String, String, Option<String>)> = sqlx::query_as("SELECT target, rights, expires FROM machine_grants WHERE machine_id = ? ORDER BY target")
            .bind(machine_id)
            .fetch_all(&s.db)
            .await
            .map_err(err)?;
        for (target, rights, expires) in rows.into_iter().filter(|r| covers(&r.0, folder)) {
            let rights: Vec<String> = serde_json::from_str(&rights).unwrap_or_default();
            grants.push(json!({ "target": target, "rights": rights, "expires": expires }));
        }
    }
    let mcp: Vec<(String, String, String)> = sqlx::query_as("SELECT server, tool, expires FROM mcp_grants WHERE user_id = ? AND expires > ? ORDER BY server, tool")
        .bind(user_id)
        .bind(util::now())
        .fetch_all(&s.db)
        .await
        .map_err(err)?;
    let knowledge: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.id, c.name FROM knowledge_project kp JOIN knowledge_collection c ON c.id = kp.collection_id WHERE kp.project_id = ? ORDER BY c.name",
    )
    .bind(&p.id)
    .fetch_all(&s.db)
    .await
    .map_err(err)?;
    let assets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_asset WHERE project_id = ?").bind(&p.id).fetch_one(&s.db).await.map_err(err)?;
    let mut commands = Vec::new();
    if p.repo_machine_id.is_some() {
        let dev = devsettings::Settings::parse(&p.dev_settings);
        for kind in ["build", "test", "run", "deploy"] {
            if let Some(cmd) = dev.command(kind) {
                commands.push(json!({ "kind": kind, "command": cmd, "last": dev.last.get(kind), "fromHere": kind != "deploy" }));
            }
        }
    }
    Ok(json!({
        "grants": grants,
        "mcp": mcp.into_iter().map(|(server, tool, expires)| json!({ "server": server, "tool": tool, "expires": expires })).collect::<Vec<_>>(),
        "knowledge": knowledge.into_iter().map(|(id, name)| json!({ "id": id, "name": name })).collect::<Vec<_>>(),
        "assets": assets,
        "commands": commands,
    }))
}

/// Progress: counts from the tasks plus the last merged PR.
async fn progress_part(db: &SqlitePool, user_id: &str, project_id: &str) -> Result<Value, String> {
    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, COALESCE(parent_id, ''), kind, title, state FROM tasks WHERE project_id = ? AND user_id = ? ORDER BY position, rowid"
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;
    let rows: Vec<TaskRow> = rows.into_iter().map(|(id, parent_id, kind, title, state)| TaskRow { id, parent_id, kind, title, state }).collect();

    let deps: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT d.task_id, d.depends_on FROM task_deps d JOIN tasks t ON t.id = d.task_id WHERE t.project_id = ? AND t.user_id = ?"
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_all(db)
    .await.map_err(|e| e.to_string())?;

    let mut out = progress(&rows, &deps);
    out["lastMerged"] = Value::Null;

    let events: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT t.title, e.detail FROM task_events e JOIN tasks t ON t.id = e.task_id WHERE t.project_id = ? AND t.user_id = ? AND e.kind = 'forge' ORDER BY e.id DESC LIMIT 20"
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_all(db)
    .await.map_err(|e| e.to_string())?;

    for (title, detail) in events {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&detail) {
            if let Some(state) = v.get("state").and_then(|s| s.as_object()) {
                if state.get("to").and_then(|t| t.as_str()) == Some("done") {
                    if let Some(pr) = v.get("pr").and_then(|p| p.as_str()) {
                        out["lastMerged"] = json!({ "title": title, "pr": pr });
                        break;
                    }
                }
            }
        }
    }

    Ok(out)
}

/// The last 10 events across task events, runs and the project's chats, newest first.
async fn activity_part(db: &SqlitePool, user_id: &str, project_id: &str) -> Result<Value, String> {
    let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT 'task', e.kind, t.title, e.at, e.detail, '' FROM task_events e JOIN tasks t ON t.id = e.task_id WHERE t.project_id = ? AND t.user_id = ?
         UNION ALL SELECT 'run', r.status, t.title, COALESCE(r.ended_at, r.started_at), '', COALESCE((SELECT c.model_id FROM calls c WHERE c.run_id = r.id GROUP BY c.model_id ORDER BY COUNT(*) DESC LIMIT 1), '') FROM runs r JOIN tasks t ON t.id = r.task_id WHERE t.project_id = ? AND t.user_id = ?
         UNION ALL SELECT 'chat', '', title, updated_at, '', '' FROM chats WHERE project_id = ? AND user_id = ?
         ORDER BY 4 DESC LIMIT 10"
    )
    .bind(project_id)
    .bind(user_id)
    .bind(project_id)
    .bind(user_id)
    .bind(project_id)
    .bind(user_id)
    .fetch_all(db)
    .await.map_err(|e| e.to_string())?;

    let mut out: Vec<Value> = Vec::new();
    for (src, kind, title, at, detail, who) in rows {
        out.push(json!({ "what": activity_what(&src, &kind, &title, &detail), "at": at, "who": who }));
    }
    Ok(serde_json::Value::Array(out))
}

/// Models used by the project's runs in the last 30 days: runs, tokens and model minutes per model.
async fn ai_part(db: &SqlitePool, user_id: &str, project_id: &str) -> Result<Value, String> {
    let cutoff = util::minutes_ago(30 * 24 * 60);
    let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
        "SELECT c.model_id, COUNT(DISTINCT r.id), COALESCE(SUM(COALESCE(c.tokens_in, 0) + COALESCE(c.tokens_out, 0)), 0), COALESCE(SUM(c.ms), 0) FROM calls c JOIN runs r ON r.id = c.run_id JOIN tasks t ON t.id = r.task_id WHERE t.project_id = ? AND t.user_id = ? AND r.started_at >= ? GROUP BY c.model_id ORDER BY 3 DESC"
    )
    .bind(project_id)
    .bind(user_id)
    .bind(&cutoff)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    let mut arr: Vec<Value> = Vec::new();
    for (model, runs, tokens, ms) in rows {
        let minutes = ((ms as f64 / 60000.0) * 10.0).round() / 10.0;
        arr.push(json!({ "model": model, "runs": runs, "tokens": tokens, "minutes": minutes }));
    }
    Ok(Value::Array(arr))
}

/// GET /api/projects/{id}/overview (owner only; anyone else gets 404).
pub async fn get(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let p = load(&s.db, &u.id, &id).await?.ok_or(ApiError::NotFound)?;

    let mut errors = serde_json::Map::new();
    let where_part_res = where_part(&s, &u.id, &p).await;
    let access_part_res = access_part(&s, &u.id, &p).await;
    let progress_part_res = progress_part(&s.db, &u.id, &p.id).await;
    let activity_part_res = activity_part(&s.db, &u.id, &p.id).await;
    let ai_part_res = ai_part(&s.db, &u.id, &p.id).await;

    if let Err(ref reason) = where_part_res {
        errors.insert("where".to_string(), Value::String(reason.clone()));
    }
    if let Err(ref reason) = access_part_res {
        errors.insert("access".to_string(), Value::String(reason.clone()));
    }
    if let Err(ref reason) = progress_part_res {
        errors.insert("progress".to_string(), Value::String(reason.clone()));
    }
    if let Err(ref reason) = activity_part_res {
        errors.insert("activity".to_string(), Value::String(reason.clone()));
    }
    if let Err(ref reason) = ai_part_res {
        errors.insert("ai".to_string(), Value::String(reason.clone()));
    }

    let cfg = scheduler::Config::parse(&p.scheduler);
    let automation = json!({ "scheduler": { "enabled": cfg.enabled, "budget": cfg.budget }, "overseer": p.overseer });

    let links = serde_json::from_str::<Value>(&p.links).ok().filter(Value::is_array).unwrap_or_else(|| json!([]));

    Ok(Json(json!({
        "id": p.id,
        "name": p.name,
        "description": p.description,
        "about": p.about,
        "type": p.ptype,
        "kind": p.kind,
        "source": p.source,
        "links": links,
        "createdAt": p.created_at,
        "updatedAt": p.updated_at,
        "archivedAt": p.archived_at,
        "owner": p.owner,
        "where": where_part_res.ok().unwrap_or(Value::Null),
        "access": access_part_res.ok().unwrap_or(Value::Null),
        "progress": progress_part_res.ok().unwrap_or(Value::Null),
        "activity": activity_part_res.ok().unwrap_or(Value::Null),
        "ai": ai_part_res.ok().unwrap_or(Value::Null),
        "automation": automation,
        "errors": Value::Object(errors)
    })))
}

#[cfg(test)]
#[path = "overview_tests.rs"]
mod tests;
