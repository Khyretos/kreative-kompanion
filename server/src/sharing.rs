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

/// The owner's id when `user_id` may change the project's tasks and chats: its owner or an editor (a viewer or stranger gets None).
pub async fn project_editor(db: &SqlitePool, user_id: &str, project_id: &str) -> sqlx::Result<Option<String>> {
    Ok(access(db, user_id, project_id).await?.filter(|a| a.role != Role::Viewer).map(|a| a.owner_id))
}

/// The owner's id when `user_id` may change this task (see `project_editor`).
pub async fn task_editor(db: &SqlitePool, user_id: &str, task_id: &str) -> sqlx::Result<Option<String>> {
    let project: Option<String> = sqlx::query_scalar("SELECT project_id FROM tasks WHERE id = ?").bind(task_id).fetch_optional(db).await?;
    match project {
        Some(p) => project_editor(db, user_id, &p).await,
        None => Ok(None),
    }
}

/// The owner's id when `user_id` may write in this chat: it is theirs, or it belongs to a project they edit.
pub async fn chat_editor(db: &SqlitePool, user_id: &str, chat_id: &str) -> sqlx::Result<Option<String>> {
    let chat: Option<(String, Option<String>)> = sqlx::query_as("SELECT user_id, project_id FROM chats WHERE id = ?").bind(chat_id).fetch_optional(db).await?;
    match chat {
        Some((owner, _)) if owner == user_id => Ok(Some(owner)),
        Some((_, Some(project))) => project_editor(db, user_id, &project).await,
        _ => Ok(None),
    }
}

/// Returns everyone who may read this chat: the chat's owner, plus the project's owner and members if the chat belongs to a project.
pub async fn chat_audience(db: &SqlitePool, chat_id: &str) -> sqlx::Result<Vec<String>> {
    let chat: Option<(String, Option<String>)> = sqlx::query_as("SELECT user_id, project_id FROM chats WHERE id = ?").bind(chat_id).fetch_optional(db).await?;
    let mut set = std::collections::BTreeSet::new();
    let Some((owner, project_id)) = chat else { return Ok(Vec::new()); };
    set.insert(owner.clone());
    if let Some(pid) = project_id {
        // Add project owner
        let proj_owner: Option<String> = sqlx::query_scalar("SELECT user_id FROM projects WHERE id = ?").bind(&pid).fetch_optional(db).await?;
        if let Some(uid) = proj_owner { set.insert(uid); }
        // Add project members
        let members: Vec<String> = sqlx::query_scalar("SELECT user_id FROM project_members WHERE project_id = ?")
            .bind(&pid)
            .fetch_all(db)
            .await?;
        for m in members { set.insert(m); }
    }
    Ok(set.into_iter().collect())
}

/// Returns the user plus everyone who shares at least one project with them: owners of projects the user is a member of, and members of projects the user owns or is a member of.
pub async fn collaborators(db: &SqlitePool, user_id: &str) -> sqlx::Result<Vec<String>> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT p.user_id FROM projects p JOIN project_members m ON m.project_id = p.id WHERE m.user_id = ?1 OR p.user_id = ?1 UNION SELECT m2.user_id FROM project_members m2 JOIN projects p2 ON p2.id = m2.project_id LEFT JOIN project_members me ON me.project_id = p2.id AND me.user_id = ?1 WHERE p2.user_id = ?1 OR me.user_id IS NOT NULL",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;
    let mut set = std::collections::BTreeSet::new();
    set.insert(user_id.to_string());
    for r in rows { set.insert(r); }
    Ok(set.into_iter().collect())
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
