//! STU-D1: delete finished Studio runs and their files. A file that Assets still uses (STU-02b
//! registers the same path, no copy) stays on disk; paths outside the output dir are never touched.
use std::path::Path;
use axum::{Extension, Json, extract::{Path as UrlPath, State}, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use crate::{AppState, auth::User, error::{ApiError, ApiResult}};

const FINISHED: [&str; 4] = ["done", "failed", "dropped", "cancelled"];

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct Removed {
    pub deleted: Vec<String>,
    pub skipped: Vec<String>,
    #[serde(rename = "keptFiles")]
    pub kept_files: usize,
}

pub async fn remove_runs(
    db: &SqlitePool,
    root: &Path,
    user_id: &str,
    admin: bool,
    ids: &[String],
) -> sqlx::Result<Removed> {
    let root_canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    let mut deleted = Vec::new();
    let mut skipped = Vec::new();
    let mut kept_files = 0usize;

    for id in ids {
        let row = sqlx::query_as::<_, (String, String, String)>(
            "SELECT user_id, state, outputs FROM studio_run WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(db)
        .await?;
        let Some(row) = row else {
            skipped.push(id.clone());
            continue;
        };

        let owner = row.0;
        let state = row.1;
        let outputs_str = row.2;

        if owner != *user_id && !admin {
            skipped.push(id.clone());
            continue;
        }

        if !FINISHED.contains(&state.as_str()) {
            skipped.push(id.clone());
            continue;
        }

        let outputs: Vec<String> = serde_json::from_str(&outputs_str)
            .unwrap_or_default();

        for path_str in outputs {
            match std::fs::canonicalize(&path_str) {
                Ok(p) => {
                    if !p.starts_with(&root_canonical) {
                        continue;
                    }

                    let count: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM asset WHERE path IN (?, ?)"
                    )
                    .bind(p.to_string_lossy().to_string())
                    .bind(&path_str)
                    .fetch_one(db)
                    .await?;

                    if count > 0 {
                        kept_files += 1;
                    } else {
                        let _ = std::fs::remove_file(&p);
                    }
                }
                Err(_) => continue,
            }
        }

        sqlx::query("DELETE FROM studio_run WHERE id = ?")
            .bind(id)
            .execute(db)
            .await?;

        deleted.push(id.clone());
    }

    Ok(Removed {
        deleted,
        skipped,
        kept_files,
    })
}

pub async fn failed_ids(db: &SqlitePool, user_id: &str) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar::<_, String>(
        "SELECT id FROM studio_run WHERE user_id = ? AND state = 'failed'"
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

#[derive(Deserialize)]
pub struct DeleteMany {
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub failed: bool,
}

pub async fn delete_one(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    UrlPath(id): UrlPath<String>,
) -> ApiResult<StatusCode> {
    let admin = crate::admin::is_admin(&s.db, &u.id).await?;
    let r = remove_runs(&s.db, &s.config.studio.output_dir, &u.id, admin, &[id.clone()]).await?;

    if !r.deleted.is_empty() {
        return Ok(StatusCode::NO_CONTENT);
    }

    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT user_id, state FROM studio_run WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await?
    .ok_or(ApiError::NotFound)?;

    if row.0 != u.id && !admin {
        return Err(ApiError::NotFound);
    }

    if row.1 != "done" && row.1 != "failed" && row.1 != "dropped" && row.1 != "cancelled" {
        return Err(ApiError::BadRequest("This run is still going. Stop it first, then delete it.".into()));
    }

    Err(ApiError::NotFound)
}

pub async fn delete_many(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Json(b): Json<DeleteMany>,
) -> ApiResult<Json<Removed>> {
    let mut ids = b.ids;
    if b.failed {
        let failed = failed_ids(&s.db, &u.id).await?;
        ids.extend(failed);
    }
    ids.sort();
    ids.dedup();

    if ids.len() > 500 {
        return Err(ApiError::BadRequest("Delete at most 500 runs at once.".into()));
    }

    let admin = crate::admin::is_admin(&s.db, &u.id).await?;
    Ok(Json(remove_runs(&s.db, &s.config.studio.output_dir, &u.id, admin, &ids).await?))
}

#[cfg(test)]
#[path = "remove_tests.rs"]
mod tests;
