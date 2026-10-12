//! PROJ-OVERVIEW: GET /api/projects/{id}/overview, one call with what a project's Overview shows: what it is,
//! where it lives, what it may reach, how far along it is, what happened lately and which models worked on it.
//! Every part comes from existing tables; a part that fails is `null` with its reason in `errors`, never a 500.
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
#[derive(Debug, Clone)]
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
    todo!("unit o3")
}

/// pure: the project text a prompt gets: the description, then at most `max` chars of `about`.
pub fn with_about(description: &str, about: &str, max: usize) -> String {
    todo!("unit o4")
}

/// pure: one activity line. `src` is "task" (a task event), "run" (a task run) or "chat".
pub fn activity_what(src: &str, kind: &str, title: &str, detail: &str) -> String {
    todo!("unit o5")
}

/// pure: progress counts from the task rows; `deps` holds (task_id, depends_on) pairs.
pub fn progress(rows: &[TaskRow], deps: &[(String, String)]) -> Value {
    todo!("unit o6")
}

/// pure: the remote and branch from the output of `git remote get-url origin; git branch --show-current`.
pub fn parse_git(output: &str) -> Option<(String, String)> {
    todo!("unit o7")
}

/// Reads the repo's git remote and branch on its computer (a shell job under the computer's grants) and
/// caches them in `projects.repo_git`. Runs in the background; a failure leaves the cache as it was.
pub async fn refresh_git(s: AppState, user_id: String, project_id: String, machine_id: String, folder: String) {
    todo!("unit o8")
}

/// The project row with its owner's name; None when it is not this user's.
pub async fn load(db: &SqlitePool, user_id: &str, project_id: &str) -> sqlx::Result<Option<ProjectRow>> {
    todo!("unit o9")
}

/// Where it lives: the repo computer, the folder and the cached git remote and branch.
async fn where_part(s: &AppState, user_id: &str, p: &ProjectRow) -> Result<Value, String> {
    todo!("unit o10")
}

/// What it can reach: computer grants covering the repo folder, active MCP grants, knowledge, assets, commands.
async fn access_part(s: &AppState, user_id: &str, p: &ProjectRow) -> Result<Value, String> {
    todo!("unit o11")
}

/// Progress: counts from the tasks plus the last merged PR.
async fn progress_part(db: &SqlitePool, user_id: &str, project_id: &str) -> Result<Value, String> {
    todo!("unit o12")
}

/// The last 10 events across task events, runs and the project's chats, newest first.
async fn activity_part(db: &SqlitePool, user_id: &str, project_id: &str) -> Result<Value, String> {
    todo!("unit o13")
}

/// Models used by the project's runs in the last 30 days: runs, tokens and model minutes per model.
async fn ai_part(db: &SqlitePool, user_id: &str, project_id: &str) -> Result<Value, String> {
    todo!("unit o14")
}

/// GET /api/projects/{id}/overview (owner only; anyone else gets 404).
pub async fn get(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    todo!("unit o15")
}

#[cfg(test)]
#[path = "overview_tests.rs"]
mod tests;
