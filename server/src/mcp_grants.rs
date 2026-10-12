//! KG-05: MCP tools inside task runs. A worker is offered the tools of the `[[mcp]]` servers the
//! user has granted (`mcp_grants`, per server or per tool, with an expiry), and every call is
//! checked against the grants again when it is made: without a grant the tool is not offered and
//! a call is denied without reaching the server. Grants are managed over the API below.
use std::collections::HashMap;

use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{AppState, auth::User, error::{ApiError, ApiResult}, util};

/// The longest a grant may last: a week.
const MAX_HOURS: i64 = 24 * 7;

/// Whether grants on one server allow one tool: a "*" grant allows every tool, others one tool.
pub fn allows(grants: &[(String, String)], server: &str, tool: &str) -> bool {
    grants.iter().any(|(s, t)| s == server && (t == "*" || t == tool))
}

/// The user's unexpired grants as (server, tool).
async fn current(s: &AppState, user_id: &str) -> Vec<(String, String)> {
    sqlx::query_as("SELECT server, tool FROM mcp_grants WHERE user_id = ? AND expires > ?")
        .bind(user_id)
        .bind(util::now())
        .fetch_all(&s.db)
        .await
        .unwrap_or_default()
}

/// The function specs a task's worker is offered, and the map from function name to (server, tool):
/// only what the user's grants allow, from enabled servers that answer.
pub async fn tools_for(s: &AppState, user_id: &str) -> (Vec<Value>, HashMap<String, (String, String)>) {
    let grants = current(s, user_id).await;
    let mut listed = Vec::new();
    for srv in s.config.mcp.iter().filter(|x| x.enabled && grants.iter().any(|(g, _)| *g == x.name)) {
        if let Ok(tools) = crate::mcp::tools_cached(&s.http, srv).await {
            let allowed: Vec<Value> = tools.into_iter().filter(|t| allows(&grants, &srv.name, t["name"].as_str().unwrap_or(""))).collect();
            if !allowed.is_empty() {
                listed.push((srv.name.clone(), allowed));
            }
        }
    }
    let (specs, map) = crate::chat_tools::tool_specs(&listed);
    (specs.as_array().cloned().unwrap_or_default(), map)
}

/// Runs one MCP tool call of a task step: denied without a grant, else the server's answer as text.
pub async fn call(s: &AppState, user_id: &str, map: &HashMap<String, (String, String)>, name: &str, args: &Value) -> String {
    let Some((server, tool)) = map.get(name) else {
        return "Unknown tool or wrong arguments.".to_string();
    };
    if !allows(&current(s, user_id).await, server, tool) {
        return format!("Denied: there is no grant for {tool} on {server}. Stop and say that the user must allow it first.");
    }
    let Some(cfg) = s.config.mcp.iter().find(|x| x.enabled && x.name == *server) else {
        return format!("error: {server} is not available");
    };
    match tokio::time::timeout(std::time::Duration::from_secs(120), crate::mcp::call_tool_full(&s.http, cfg, tool, args)).await {
        Ok(Ok((text, _, _))) => text,
        Ok(Err(e)) => format!("error: {e:#}"),
        Err(_) => "error: the tool took too long".to_string(),
    }
}

// ---- API ----

#[derive(Deserialize)]
pub struct GrantBody {
    pub server: String,
    /// One tool, or all of the server's ("*", the default).
    #[serde(default)]
    pub tool: Option<String>,
    /// How long the grant lasts, 1 hour to a week; 24 when not given.
    #[serde(default)]
    pub hours: Option<i64>,
}

/// GET /api/mcp-grants: the user's unexpired grants.
pub async fn list(State(s): State<AppState>, Extension(u): Extension<User>) -> ApiResult<Json<Value>> {
    let rows: Vec<(String, String, String, String, String)> =
        sqlx::query_as("SELECT id, server, tool, granted_at, expires FROM mcp_grants WHERE user_id = ? AND expires > ? ORDER BY granted_at")
            .bind(&u.id)
            .bind(util::now())
            .fetch_all(&s.db)
            .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(id, server, tool, granted_at, expires)| json!({ "id": id, "server": server, "tool": tool, "grantedAt": granted_at, "expires": expires }))
        .collect::<Vec<_>>())))
}

/// POST /api/mcp-grants: allow task runs to use a configured MCP server (or one of its tools).
pub async fn create(State(s): State<AppState>, Extension(u): Extension<User>, Json(b): Json<GrantBody>) -> ApiResult<Json<Value>> {
    if !s.config.mcp.iter().any(|x| x.name == b.server) {
        return Err(ApiError::BadRequest(format!("{} is not a configured MCP server.", b.server)));
    }
    let tool = b.tool.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| "*".into());
    let hours = b.hours.unwrap_or(24).clamp(1, MAX_HOURS);
    let id = util::new_id();
    let expires = util::in_hours(hours);
    // One grant per server and tool: asking again renews it.
    sqlx::query("DELETE FROM mcp_grants WHERE user_id = ? AND server = ? AND tool = ?")
        .bind(&u.id).bind(&b.server).bind(&tool).execute(&s.db).await?;
    sqlx::query("INSERT INTO mcp_grants (id, user_id, server, tool, granted_at, expires) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(&u.id).bind(&b.server).bind(&tool).bind(util::now()).bind(&expires)
        .execute(&s.db)
        .await?;
    Ok(Json(json!({ "id": id, "server": b.server, "tool": tool, "expires": expires })))
}

/// DELETE /api/mcp-grants/{id}: take a grant back.
pub async fn revoke(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    let done = sqlx::query("DELETE FROM mcp_grants WHERE id = ? AND user_id = ?").bind(&id).bind(&u.id).execute(&s.db).await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
#[path = "mcp_grants_tests.rs"]
mod tests;
