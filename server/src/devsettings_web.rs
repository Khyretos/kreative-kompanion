//! KG-10: the settings API. GET/PUT /projects/{id}/dev-settings read and set a project's repo, build,
//! test, run and deploy settings (any project type), POST /projects/{id}/dev-settings/run/{build|test|run}
//! runs one of them on the project's computer under its grants and stores the result.
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde_json::{Value, json};
use std::time::Duration;

use crate::{
    AppState,
    auth::User,
    devsettings::{self, Settings},
    error::{ApiError, ApiResult},
    events::Event,
    scheduler::{self, Verdict},
    util,
};

type Row = (String, Option<String>, Option<String>);

async fn load(s: &AppState, user_id: &str, id: &str) -> ApiResult<Row> {
    let row: Option<Row> = sqlx::query_as("SELECT dev_settings, repo_machine_id, repo_folder FROM projects WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(&s.db)
        .await?;
    row.ok_or(ApiError::NotFound)
}

fn view(dev: &Settings, machine: &Option<String>, folder: &Option<String>) -> Value {
    json!({ "settings": dev, "repoMachineId": machine, "repoFolder": folder })
}

/// GET /projects/{id}/dev-settings: the settings, the repo they run in and the presets to start from.
pub async fn get(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let (text, machine, folder) = load(&s, &u.id, &id).await?;
    let mut v = view(&Settings::parse(&text), &machine, &folder);
    v["presets"] = json!(devsettings::presets().into_iter().map(|(_, p)| p).collect::<Vec<_>>());
    Ok(Json(v))
}

/// PUT /projects/{id}/dev-settings: replaces the settings (the last results stay as they were).
pub async fn put(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Json(mut b): Json<Settings>) -> ApiResult<Json<Value>> {
    let (text, machine, folder) = load(&s, &u.id, &id).await?;
    b.validate().map_err(ApiError::BadRequest)?;
    b.last = Settings::parse(&text).last;
    let stored = serde_json::to_string(&b).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    sqlx::query("UPDATE projects SET dev_settings = ? WHERE id = ?").bind(stored).bind(&id).execute(&s.db).await?;
    s.bus.send(&u.id, Event::Changed { what: "projects", machine_id: None });
    Ok(Json(view(&b, &machine, &folder)))
}

/// POST /projects/{id}/dev-settings/run/{kind}: starts build, test or run in the repo folder (answers 202 at once;
/// the result lands in the settings' `last` and a projects event is sent). Deploy is never started from here.
pub async fn run(State(s): State<AppState>, Extension(u): Extension<User>, Path((id, kind)): Path<(String, String)>) -> ApiResult<(StatusCode, Json<Value>)> {
    let (text, machine, folder) = load(&s, &u.id, &id).await?;
    if !matches!(kind.as_str(), "build" | "test" | "run") {
        return Err(ApiError::BadRequest("Only build, test and run can be started here. Deploy is done by hand.".into()));
    }
    let dev = Settings::parse(&text);
    let cmd = dev.command(&kind).ok_or_else(|| ApiError::BadRequest(format!("Set a {kind} command first.")))?.to_string();
    let (Some(machine), Some(folder)) = (machine.filter(|m| !m.is_empty()), folder.filter(|f| !f.is_empty())) else {
        return Err(ApiError::BadRequest("Pick the computer and folder of the repo first.".into()));
    };
    let (state, user_id, started) = (s.clone(), u.id.clone(), kind.clone());
    tokio::spawn(async move {
        let verdict = scheduler::run_check(&state, &user_id, &machine, &folder, &cmd, Duration::from_secs(900)).await;
        let (ok, summary) = match verdict {
            Verdict::Pass => (true, "passed".to_string()),
            Verdict::Fail(m) | Verdict::NotRun(m) => (false, m),
        };
        let text: Option<String> = sqlx::query_scalar("SELECT dev_settings FROM projects WHERE id = ?").bind(&id).fetch_optional(&state.db).await.ok().flatten();
        let mut dev = Settings::parse(&text.unwrap_or_default());
        dev.record(&kind, ok, &util::now(), &summary);
        if let Ok(stored) = serde_json::to_string(&dev) {
            let _ = sqlx::query("UPDATE projects SET dev_settings = ? WHERE id = ?").bind(stored).bind(&id).execute(&state.db).await;
        }
        state.bus.send(&user_id, Event::Changed { what: "projects", machine_id: None });
    });
    Ok((StatusCode::ACCEPTED, Json(json!({ "started": started }))))
}

/// The planner's text about a project's repo and commands ("" when nothing is set).
pub async fn brief_of(db: &sqlx::SqlitePool, project_id: &str, user_id: &str) -> String {
    let row: Option<Row> = sqlx::query_as("SELECT dev_settings, repo_machine_id, repo_folder FROM projects WHERE id = ? AND user_id = ?")
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    row.map(|(text, _, folder)| Settings::parse(&text).brief(folder.as_deref().filter(|f| !f.is_empty()))).unwrap_or_default()
}

/// Fills the scheduler's config from the project settings where the scheduler has no value of its own.
pub async fn fill_scheduler(db: &sqlx::SqlitePool, project_id: &str, user_id: &str, cfg: &mut scheduler::Config) {
    let row: Option<Row> = sqlx::query_as("SELECT dev_settings, repo_machine_id, repo_folder FROM projects WHERE id = ? AND user_id = ?")
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    if let Some((text, machine, folder)) = row {
        devsettings::apply_to_scheduler(cfg, &Settings::parse(&text), machine.as_deref().filter(|m| !m.is_empty()), folder.as_deref().filter(|f| !f.is_empty()));
    }
}
