//! CHAT-03b: the app's knowledge API: collections, uploads into them, links to projects.

use axum::{Extension, Json, body::Bytes, extract::{Path, Query, State}, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use crate::{AppState, auth::User, error::{ApiError, ApiResult}, knowledge};

/// Whether the collection belongs to this user.
async fn owns(s: &AppState, id: &str, u: &User) -> ApiResult<bool> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM knowledge_collection WHERE id = ? AND user_id = ?").bind(id).bind(&u.id).fetch_optional(&s.db).await?;
    Ok(row.is_some())
}

/// List the user's collections.
pub async fn list(State(s): State<AppState>, Extension(u): Extension<User>) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"collections": knowledge::list(&s.db, &u.id).await?})))
}

#[derive(Deserialize)]
pub struct NewCollection { name: String }

/// Create a new collection.
pub async fn create(State(s): State<AppState>, Extension(u): Extension<User>, Json(b): Json<NewCollection>) -> ApiResult<(StatusCode, Json<Value>)> {
    let name: String = b.name.trim().chars().take(80).collect();
    if name.is_empty() { return Err(ApiError::BadRequest("Give the collection a name.".into())); }
    let id = knowledge::ensure_collection(&s.db, &u.id, &name, "app").await?;
    Ok((StatusCode::CREATED, Json(json!({"id": id, "name": name}))))
}

/// Delete a collection.
pub async fn delete(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    if knowledge::delete_collection(&s.db, &u.id, &id).await? { Ok(StatusCode::NO_CONTENT) } else { Err(ApiError::NotFound) }
}

#[derive(Deserialize)]
pub struct Projects { projects: Vec<String> }

/// Set the projects linked to a collection.
pub async fn set_projects(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Json(b): Json<Projects>) -> ApiResult<StatusCode> {
    if !owns(&s, &id, &u).await? { return Err(ApiError::NotFound); }
    knowledge::set_projects(&s.db, &u.id, &id, &b.projects).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct Upload { name: String }

/// Upload a document to a collection.
pub async fn upload(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Query(q): Query<Upload>, body: Bytes) -> ApiResult<(StatusCode, Json<Value>)> {
    if !owns(&s, &id, &u).await? { return Err(ApiError::NotFound); }
    // A re-import replaces what is in Open WebUI's and a folder's collections, so uploads go into the app's own.
    let (source,): (String,) = sqlx::query_as("SELECT source FROM knowledge_collection WHERE id = ?").bind(&id).fetch_one(&s.db).await?;
    if source == "openwebui" {
        return Err(ApiError::BadRequest("This collection is kept in step with Open WebUI. Add the file there, or to a collection made here.".into()));
    }
    if source == "folder" {
        return Err(ApiError::BadRequest("This collection is kept in step with a folder on the server. Add the file to that folder, or to a collection made here.".into()));
    }
    let name: String = q.name.trim().rsplit(['/', '\\']).next().unwrap_or("").chars().take(200).collect();
    if name.is_empty() { return Err(ApiError::BadRequest("The file needs a name.".into())); }
    if body.is_empty() { return Err(ApiError::BadRequest("The file is empty.".into())); }
    let text = knowledge::extract(&name, &body).await.map_err(|e| ApiError::BadRequest(e.to_string()))?;
    if text.trim().is_empty() { return Err(ApiError::BadRequest("No text found in the file.".into())); }
    let old: Option<(i64,)> = sqlx::query_as("SELECT id FROM knowledge_doc WHERE collection_id = ? AND name = ?").bind(&id).bind(&name).fetch_optional(&s.db).await?;
    let replaced = old.is_some();
    if let Some((old_id,)) = old { knowledge::delete_doc(&s.db, &u.id, old_id).await?; }
    let doc = knowledge::add_doc(&s.db, &id, &name, &text).await?;
    let (chunks,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM knowledge_chunk WHERE doc_id = ?").bind(doc).fetch_one(&s.db).await?;
    Ok((StatusCode::CREATED, Json(json!({"id": doc, "name": name, "chars": text.chars().count(), "chunks": chunks, "replaced": replaced}))))
}

/// Delete a document.
pub async fn delete_doc(State(s): State<AppState>, Extension(u): Extension<User>, Path((_collection, doc)): Path<(String, i64)>) -> ApiResult<StatusCode> {
    if knowledge::delete_doc(&s.db, &u.id, doc).await? { Ok(StatusCode::NO_CONTENT) } else { Err(ApiError::NotFound) }
}

#[derive(Deserialize)]
pub struct DocsQuery { #[serde(default)] q: String, #[serde(default)] offset: i64 }

/// One page of a collection's documents (50), optionally only those whose name contains `q`.
pub async fn docs(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Query(q): Query<DocsQuery>) -> ApiResult<Json<Value>> {
    let (documents, total) = knowledge::docs_page(&s.db, &u.id, &id, q.q.trim(), q.offset).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(json!({"documents": documents, "total": total})))
}

/// A document's text, to read it in the app.
pub async fn doc(State(s): State<AppState>, Extension(u): Extension<User>, Path((_collection, doc)): Path<(String, i64)>) -> ApiResult<Json<Value>> {
    let (name, text) = knowledge::doc_text(&s.db, &u.id, doc).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(json!({"id": doc, "name": name, "text": text})))
}
