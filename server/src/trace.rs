//! TASK-OVERVIEW: a task run's trace for the live drill-down: every model call (model, provider,
//! tokens, time, what it got and what it answered) and every step, in order. It is read from what
//! Kompanion records anyway, so it costs no tokens. Stored bodies are capped, kept 30 days, and
//! token-looking strings are masked before anything is shown.

use axum::{Extension, Json, extract::{Path, State}};
use serde_json::{Value, json};

use crate::{AppState, auth::User, error::{ApiError, ApiResult}};

/// Bytes of a call's request or response kept in `calls`.
pub const BODY_CAP: usize = 32 * 1024;
/// Characters of a body sent to the app (the rest is said to be cut).
const SHOWN_CAP: usize = 12_000;
/// Days call bodies are kept; the numbers (tokens, time, model) stay.
pub const KEEP_DAYS: i64 = 30;

/// A request or response as stored: the JSON itself, or, when longer than BODY_CAP, its start
/// with a note (still valid JSON).
pub fn capped(v: &Value) -> String {
    let s = v.to_string();
    if s.len() <= BODY_CAP {
        return s;
    }
    let mut end = BODY_CAP;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    json!({ "cut": true, "bytes": s.len(), "start": &s[..end] }).to_string()
}

/// A stored body as readable text: masked, and shortened for the app.
fn shown(stored: &str) -> String {
    let v: Value = serde_json::from_str(stored).unwrap_or(Value::String(stored.to_string()));
    let text = match &v {
        Value::Object(o) if o.get("cut") == Some(&json!(true)) => format!("[cut: {} bytes in all, the start follows]\n{}", o.get("bytes").and_then(Value::as_u64).unwrap_or(0), o.get("start").and_then(Value::as_str).unwrap_or("")),
        Value::String(t) => t.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    };
    let masked = crate::secrets::mask(&text);
    if masked.chars().count() <= SHOWN_CAP {
        return masked;
    }
    let head: String = masked.chars().take(SHOWN_CAP).collect();
    format!("{head}\n[shortened here: {} characters in all]", masked.chars().count())
}

/// GET /api/runs/{id}/trace: the run, its "now doing" line and its calls and steps in order.
pub async fn get(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    type Run = (String, String, String, Option<String>, String, Option<String>, Option<String>);
    let run: Option<Run> = sqlx::query_as(
        "SELECT r.task_id, t.title, r.status, r.step, r.started_at, r.ended_at, p.name FROM runs r JOIN tasks t ON t.id = r.task_id
         LEFT JOIN projects p ON p.id = t.project_id WHERE r.id = ? AND r.user_id = ?",
    )
    .bind(&id)
    .bind(&u.id)
    .fetch_optional(&s.db)
    .await?;
    let Some((task_id, title, status, step, started, ended, project)) = run else { return Err(ApiError::NotFound) };
    type Call = (String, String, String, String, String, Option<i64>, Option<i64>, i64, Option<String>, String, String, String);
    let calls: Vec<Call> = sqlx::query_as(
        "SELECT id, at, role, provider_id, model_id, tokens_in, tokens_out, ms, error, reason, request, response FROM calls WHERE run_id = ? ORDER BY at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await?;
    type Step = (String, String, String, String, Option<String>, String, Option<String>, Option<String>, String);
    let steps: Vec<Step> = sqlx::query_as(
        "SELECT id, machine_id, summary, state, result, created_at, COALESCE(decided_at, created_at), ended_at, tool FROM pc_actions WHERE run_id = ? ORDER BY created_at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await?;
    let mut items: Vec<(String, Value)> = calls
        .into_iter()
        .map(|(cid, at, role, provider, model, tin, tout, ms, error, reason, request, response)| {
            let v = json!({ "kind": "call", "id": cid, "at": at, "role": role, "provider": provider, "model": model,
                "tokensIn": tin, "tokensOut": tout, "ms": ms, "error": error.as_deref().map(crate::secrets::mask), "reason": reason,
                "prompt": shown(&request), "response": shown(&response) });
            (at, v)
        })
        .collect();
    let mut now_doing = step.clone().unwrap_or_default();
    for (sid, machine, summary, state, result, created, started_at, ended_at, tool) in steps {
        let tool: Value = serde_json::from_str(&crate::secrets::mask(&tool)).unwrap_or(Value::Null);
        if ["running", "approved", "always", "granting", "pending"].contains(&state.as_str()) {
            now_doing = summary.clone();
        }
        let v = json!({ "kind": "step", "id": sid, "machineId": machine, "summary": crate::secrets::mask(&summary), "state": state,
            "result": result.as_deref().map(crate::secrets::mask), "createdAt": created, "startedAt": started_at, "endedAt": ended_at, "tool": tool });
        items.push((created, v));
    }
    items.sort_by(|a, b| a.0.cmp(&b.0));
    if now_doing.is_empty() {
        now_doing = items.iter().rev().find_map(|(_, v)| v["summary"].as_str().map(str::to_string)).unwrap_or_default();
    }
    Ok(Json(json!({
        "run": { "id": id, "taskId": task_id, "title": title, "project": project, "status": status, "startedAt": started, "endedAt": ended },
        "nowDoing": crate::secrets::mask(&now_doing),
        "items": items.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
    })))
}

/// Call bodies older than KEEP_DAYS are emptied (the numbers stay), once a day.
pub fn spawn_retention(db: sqlx::SqlitePool) {
    crate::util::supervise("trace-retention", move || {
        let db = db.clone();
        async move {
            loop {
                prune(&db, time::OffsetDateTime::now_utc()).await;
                tokio::time::sleep(std::time::Duration::from_secs(24 * 3600)).await;
            }
        }
    });
}

pub async fn prune(db: &sqlx::SqlitePool, now: time::OffsetDateTime) -> u64 {
    let before = (now - time::Duration::days(KEEP_DAYS)).format(&time::format_description::well_known::Rfc3339).unwrap_or_default();
    sqlx::query("UPDATE calls SET request = '{}', response = '\"\"' WHERE at < ? AND (request <> '{}' OR response <> '\"\"')")
        .bind(before)
        .execute(db)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
