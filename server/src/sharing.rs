//! Project sharing: roles, access checks, adding and removing members.
use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::{AppState, auth::User, error::{ApiError, ApiResult}, events::Event, util};

/// The permission level a user has in a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Owner,
    Editor,
    Viewer,
}

impl Role {
    /// Returns the role as a lowercase string.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Editor => "editor",
            Role::Viewer => "viewer",
        }
    }
}

/// Access details for a user in a project.
pub struct Access {
    pub owner_id: String,
    pub role: Role,
}

/// GET /projects/{id}/access: check if the user has access to this project.
pub async fn access(db: &SqlitePool, user_id: &str, project_id: &str) -> sqlx::Result<Option<Access>> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT p.user_id, m.role FROM projects p LEFT JOIN project_members m ON m.project_id = p.id AND m.user_id = ? WHERE p.id = ?",
    )
    .bind(user_id)
    .bind(project_id)
    .fetch_optional(db)
    .await?;

    let Some((owner_id, role_str)) = row else {
        return Ok(None);
    };

    if owner_id == user_id {
        return Ok(Some(Access { owner_id, role: Role::Owner }));
    }

    let role_str = role_str.as_deref();
    match role_str {
        Some("editor") => Ok(Some(Access { owner_id, role: Role::Editor })),
        Some("viewer") => Ok(Some(Access { owner_id, role: Role::Viewer })),
        _ => Ok(None),
    }
}

/// Body for adding a member to a project.
#[derive(Deserialize)]
pub struct AddBody {
    name: String,
    role: String,
}

/// POST /projects/{id}/members: add a new member to the project.
pub async fn add(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<String>,
    Json(b): Json<AddBody>,
) -> ApiResult<StatusCode> {
    let access = access(&s.db, &u.id, &id).await?;
    let Some(access) = access else {
        return Err(ApiError::NotFound);
    };

    if access.role != Role::Owner {
        return Err(ApiError::NotFound);
    }

    let role = b.role.trim().to_lowercase();
    if !matches!(role.as_str(), "viewer" | "editor") {
        return Err(ApiError::BadRequest("A member is a viewer or an editor.".into()));
    }

    let name = b.name.trim();
    let user_id: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE name = ?")
        .bind(name)
        .fetch_optional(&s.db)
        .await?;

    let Some(user_id) = user_id else {
        return Err(ApiError::BadRequest("No user with that name.".into()));
    };

    if user_id == u.id {
        return Err(ApiError::BadRequest("You already own this project.".into()));
    }

    sqlx::query(
        "INSERT INTO project_members (project_id, user_id, role, added_at) VALUES (?, ?, ?, ?) ON CONFLICT(project_id, user_id) DO UPDATE SET role = excluded.role",
    )
    .bind(&id)
    .bind(&user_id)
    .bind(role)
    .bind(util::now())
    .execute(&s.db)
    .await?;

    s.bus.send(&u.id, Event::Changed { what: "projects", machine_id: None });
    s.bus.send(&user_id, Event::Changed { what: "projects", machine_id: None });

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /projects/{id}/members/{member}: remove a member from the project.
pub async fn remove(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path((id, member)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let access = access(&s.db, &u.id, &id).await?;
    let Some(access) = access else {
        return Err(ApiError::NotFound);
    };

    if access.role != Role::Owner && member != u.id {
        return Err(ApiError::NotFound);
    }

    sqlx::query("DELETE FROM project_members WHERE project_id = ? AND user_id = ?")
        .bind(&id)
        .bind(&member)
        .execute(&s.db)
        .await?;

    s.bus.send(&u.id, Event::Changed { what: "projects", machine_id: None });
    s.bus.send(&member, Event::Changed { what: "projects", machine_id: None });

    Ok(StatusCode::NO_CONTENT)
}

/// GET /projects/{id}/members: list all members of the project.
pub async fn members(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let access = access(&s.db, &u.id, &id).await?;
    let Some(access) = access else {
        return Err(ApiError::NotFound);
    };

    let owner_name: Option<(String,)> = sqlx::query_as("SELECT name FROM users WHERE id = ?")
        .bind(&access.owner_id)
        .fetch_optional(&s.db)
        .await?;

    let owner_name = owner_name.map(|(n,)| n).unwrap_or_else(|| String::new());

    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT m.user_id, u.name, m.role FROM project_members m JOIN users u ON u.id = m.user_id WHERE m.project_id = ? ORDER BY u.name",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await?;

    let members: Vec<Value> = rows.into_iter().map(|(uid, uname, r)| {
        json!({ "userId": uid, "name": uname, "role": r })
    }).collect();

    Ok(Json(json!({
        "role": access.role.as_str(),
        "owner": { "id": access.owner_id, "name": owner_name },
        "members": members,
    })))
}

#[cfg(test)]
#[path = "sharing_tests.rs"]
mod tests;
