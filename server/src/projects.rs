//! Project types, set by hand per project (Kees, 2026-10-04): "chat" (as before), "game"
//! (assets from the library attached to it) and "programming" (a repo folder on a computer,
//! the default for W2 "Run it on a computer").
use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{AppState, auth::User, error::{ApiError, ApiResult}, events::Event, util};

const TYPES: &[&str] = &["chat", "game", "programming"];

async fn owned(s: &AppState, project_id: &str, u: &User) -> ApiResult<()> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM projects WHERE id = ? AND user_id = ?")
        .bind(project_id)
        .bind(&u.id)
        .fetch_optional(&s.db)
        .await?;
    row.map(|_| ()).ok_or(ApiError::NotFound)
}

/// An absolute folder without `..`, without the trailing slash; "" clears it.
fn clean_folder(folder: &str) -> ApiResult<String> {
    let f = folder.trim().trim_end_matches('/');
    if f.is_empty() {
        return Ok(String::new());
    }
    if !f.starts_with('/') || f.split('/').any(|c| c == "..") || f.len() > 400 {
        return Err(ApiError::BadRequest("Use an absolute folder, like /home/you/projects/app.".into()));
    }
    Ok(f.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBody {
    #[serde(rename = "type")]
    ptype: Option<String>,
    repo_folder: Option<String>,
    repo_machine_id: Option<String>,
    /// OVR-01b: the Overseer answers every chat of this project.
    overseer: Option<bool>,
    /// PROJ-ARCHIVE: true archives the project (the first date is kept), false unarchives it.
    archived: Option<bool>,
}

/// PATCH /projects/{id}/settings: the type, and for programming projects the repo.
pub async fn settings(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<String>,
    Json(b): Json<SettingsBody>,
) -> ApiResult<StatusCode> {
    owned(&s, &id, &u).await?;
    if let Some(t) = &b.ptype {
        if !TYPES.contains(&t.as_str()) {
            return Err(ApiError::BadRequest("A project is a chat, game or programming project.".into()));
        }
        sqlx::query("UPDATE projects SET ptype = ? WHERE id = ?").bind(t).bind(&id).execute(&s.db).await?;
    }
    if let Some(f) = &b.repo_folder {
        let f = clean_folder(f)?;
        sqlx::query("UPDATE projects SET repo_folder = NULLIF(?, '') WHERE id = ?").bind(f).bind(&id).execute(&s.db).await?;
    }
    if let Some(m) = b.repo_machine_id.as_deref().map(str::trim) {
        if !m.is_empty() {
            let mine: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM machines WHERE id = ? AND user_id = ?")
                .bind(m)
                .bind(&u.id)
                .fetch_optional(&s.db)
                .await?;
            if mine.is_none() {
                return Err(ApiError::BadRequest("Pick one of your computers.".into()));
            }
        }
        sqlx::query("UPDATE projects SET repo_machine_id = NULLIF(?, '') WHERE id = ?").bind(m).bind(&id).execute(&s.db).await?;
    }
    if let Some(on) = b.overseer {
        sqlx::query("UPDATE projects SET overseer = ? WHERE id = ?").bind(on).bind(&id).execute(&s.db).await?;
    }
    if let Some(archived) = b.archived {
        if archived {
            sqlx::query("UPDATE projects SET archived_at = COALESCE(archived_at, ?) WHERE id = ?").bind(util::now()).bind(&id).execute(&s.db).await?;
        } else {
            sqlx::query("UPDATE projects SET archived_at = NULL WHERE id = ?").bind(&id).execute(&s.db).await?;
        }
    }
    s.bus.send(&u.id, Event::Changed { what: "projects", machine_id: None });
    Ok(StatusCode::NO_CONTENT)
}

type AssetRow = (i64, String, String, String, Option<String>, Option<String>, i64, Option<String>);

/// GET /projects/{id}/assets: the assets attached to a game project, newest first, in the
/// fields the Assets cards use. Empty when the asset library can't be read.
pub async fn assets(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<Json<Vec<Value>>> {
    owned(&s, &id, &u).await?;
    let rows: Vec<AssetRow> = match sqlx::query_as(
        "SELECT a.id, a.name, a.category, p.name, a.preview_kind, a.preview_state, a.preview_v, a.missing_since
         FROM project_asset pa JOIN asset a ON a.id = pa.asset_id JOIN asset_pack p ON p.id = a.pack_id
         WHERE pa.project_id = ? ORDER BY pa.added_at DESC, a.id",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "project assets: the asset library can't be read");
            Vec::new()
        }
    };
    Ok(Json(
        rows.into_iter()
            .map(|(id, name, category, pack, kind, state, pv, missing)| {
                json!({
                    "id": id, "name": name, "category": category, "pack": pack,
                    "preview": if state.as_deref() == Some("ok") { kind } else { None },
                    "pv": pv, "missing": missing.is_some(),
                })
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachBody {
    asset_id: i64,
}

/// POST /projects/{id}/assets: attach a library asset (again is fine).
pub async fn attach(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Path(id): Path<String>,
    Json(b): Json<AttachBody>,
) -> ApiResult<StatusCode> {
    owned(&s, &id, &u).await?;
    let known: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM asset WHERE id = ?").bind(b.asset_id).fetch_optional(&s.db).await.ok().flatten();
    if known.is_none() {
        return Err(ApiError::NotFound);
    }
    sqlx::query("INSERT OR IGNORE INTO project_asset (project_id, asset_id, added_at) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(b.asset_id)
        .bind(util::now())
        .execute(&s.db)
        .await?;
    s.bus.send(&u.id, Event::Changed { what: "project-assets", machine_id: None });
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /projects/{id}/assets/{asset}: detach it (the asset itself stays in the library).
pub async fn detach(State(s): State<AppState>, Extension(u): Extension<User>, Path((id, asset)): Path<(String, i64)>) -> ApiResult<StatusCode> {
    owned(&s, &id, &u).await?;
    sqlx::query("DELETE FROM project_asset WHERE project_id = ? AND asset_id = ?").bind(&id).bind(asset).execute(&s.db).await?;
    s.bus.send(&u.id, Event::Changed { what: "project-assets", machine_id: None });
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn db() -> SqlitePool {
        let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        for q in [
            "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'u1', 'x', '2026'), ('u2', 'u2', 'x', '2026')",
            "INSERT INTO projects (id, name, updated_at, user_id) VALUES ('p1', 'Game', '2026', 'u1')",
            "INSERT INTO machines (id, user_id, name, token_hash, created_at) VALUES ('m1', 'u1', 'pc', 'h', '2026'), ('m2', 'u2', 'pc2', 'h2', '2026')",
            "INSERT INTO asset_pack (id, key, name, kind) VALUES (1, 'forest.zip', 'Forest pack', 'zip')",
            "INSERT INTO asset (id, pack_id, path, name, ext, size, category, rule) VALUES (7, 1, 'tree.png', 'tree.png', 'png', 10, 'sprite', 'ext')",
        ] {
            sqlx::query(q).execute(&db).await.unwrap();
        }
        db
    }

    fn user(id: &str) -> User {
        User { id: id.into(), name: id.into() }
    }

    fn state(db: SqlitePool) -> AppState {
        AppState::for_tests(toml::from_str("").unwrap(), db)
    }

    fn body(v: Value) -> Json<SettingsBody> {
        Json(serde_json::from_value(v).unwrap())
    }

    #[test]
    fn links_are_trimmed_checked_and_stored_as_json() {
        let l = |label: &str, url: &str| Link { label: label.into(), url: url.into() };
        assert_eq!(clean_links(&[l(" Repo ", " https://g.test/k "), l("Site", "http://s.test")]).unwrap(), r#"[{"label":"Repo","url":"https://g.test/k"},{"label":"Site","url":"http://s.test"}]"#);
        assert_eq!(clean_links(&[]).unwrap(), "[]");
        for bad in [l("x", "javascript:alert(1)"), l("x", "ftp://a.test"), l("x", "https://a.test/a b"), l("", "https://a.test"), l(&"x".repeat(61), "https://a.test"), l("x", &format!("https://a.test/{}", "p".repeat(500)))] {
            assert!(matches!(clean_links(&[bad]), Err(ApiError::BadRequest(_))));
        }
        let eleven: Vec<Link> = (0..11).map(|_| l("x", "https://a.test")).collect();
        assert!(clean_links(&eleven).is_err());
        assert!(clean_links(&eleven[..10]).is_ok());
    }

    /// PROJ-OVERVIEW: name, description, about and links are checked, trimmed and saved; a Windshift
    /// project keeps its name and description (they are edited in Windshift).
    #[tokio::test]
    async fn overview_fields_are_checked_and_saved() {
        let db = db().await;
        let s = state(db.clone());
        let mut events = s.bus.subscribe();
        let patch = |v: Value| settings(State(s.clone()), Extension(user("u1")), Path("p1".into()), body(v));
        patch(json!({
            "name": " Forest ", "description": " A calm game ", "about": "# Why\nBecause.",
            "links": [{ "label": " Repo ", "url": " https://git.example.com/k/forest " }, { "label": "Site", "url": "http://forest.test" }],
        }))
        .await
        .unwrap();
        assert!(matches!(events.try_recv(), Ok((ref u, crate::events::Event::Changed { what: "projects", .. })) if u == "u1"));
        let row: (String, String, String, String) = sqlx::query_as("SELECT name, description, about, links FROM projects WHERE id = 'p1'").fetch_one(&db).await.unwrap();
        assert_eq!((row.0.as_str(), row.1.as_str(), row.2.as_str()), ("Forest", "A calm game", "# Why\nBecause."));
        let links: Value = serde_json::from_str(&row.3).unwrap();
        assert_eq!(links, json!([{ "label": "Repo", "url": "https://git.example.com/k/forest" }, { "label": "Site", "url": "http://forest.test" }]));
        // An empty list clears the links; an empty about clears it.
        patch(json!({ "links": [], "about": "" })).await.unwrap();
        let row: (String, String) = sqlx::query_as("SELECT about, links FROM projects WHERE id = 'p1'").fetch_one(&db).await.unwrap();
        assert_eq!(row, (String::new(), "[]".into()));
        // Refused, and nothing changes.
        let link = |label: &str, url: &str| json!({ "links": [{ "label": label, "url": url }] });
        let eleven: Vec<Value> = (0..11).map(|i| json!({ "label": format!("L{i}"), "url": "https://a.test" })).collect();
        for bad in [
            json!({ "name": "  " }),
            json!({ "name": "x".repeat(81) }),
            json!({ "description": "two\nlines" }),
            json!({ "description": "x".repeat(201) }),
            json!({ "about": "x".repeat(20_001) }),
            json!({ "links": eleven }),
            link("Bad", "javascript:alert(1)"),
            link("Ftp", "ftp://files.test"),
            link("Space", "https://a.test/x y"),
            link("", "https://a.test"),
            link(&"x".repeat(61), "https://a.test"),
        ] {
            assert!(matches!(patch(bad.clone()).await, Err(ApiError::BadRequest(_))), "{bad}");
        }
        let name: String = sqlx::query_scalar("SELECT name FROM projects WHERE id = 'p1'").fetch_one(&db).await.unwrap();
        assert_eq!(name, "Forest");
        // 80 characters, 200 characters and 20,000 bytes are still fine.
        patch(json!({ "name": "é".repeat(80), "description": "d".repeat(200), "about": "a".repeat(20_000) })).await.unwrap();
        // Windshift projects: name and description are read-only here, about and links are not.
        sqlx::query("UPDATE projects SET kind = 'windshift' WHERE id = 'p1'").execute(&db).await.unwrap();
        assert!(matches!(patch(json!({ "name": "Renamed" })).await, Err(ApiError::BadRequest(m)) if m.contains("Windshift")));
        assert!(matches!(patch(json!({ "description": "New" })).await, Err(ApiError::BadRequest(m)) if m.contains("Windshift")));
        patch(json!({ "about": "Still editable." })).await.unwrap();
        // Someone else's project: not found.
        let other = settings(State(s.clone()), Extension(user("u2")), Path("p1".into()), body(json!({ "about": "x" })));
        assert!(matches!(other.await, Err(ApiError::NotFound)));
    }

    #[test]
    fn folders_must_be_absolute() {
        assert_eq!(clean_folder(" /home/k/app/ ").unwrap(), "/home/k/app");
        assert_eq!(clean_folder("").unwrap(), "");
        assert!(clean_folder("app").is_err());
        assert!(clean_folder("/home/../etc").is_err());
    }

    #[tokio::test]
    async fn type_and_repo_are_saved_and_checked() {
        let db = db().await;
        let s = state(db.clone());
        let body = |t: Option<&str>, f: Option<&str>, m: Option<&str>| body(json!({ "type": t, "repoFolder": f, "repoMachineId": m }));
        settings(State(s.clone()), Extension(user("u1")), Path("p1".into()), body(Some("programming"), Some("/home/k/app/"), Some("m1")))
            .await
            .unwrap();
        let row: (String, Option<String>, Option<String>) =
            sqlx::query_as("SELECT ptype, repo_folder, repo_machine_id FROM projects WHERE id = 'p1'").fetch_one(&db).await.unwrap();
        assert_eq!(row, ("programming".into(), Some("/home/k/app".into()), Some("m1".into())));
        // Someone else's computer, an unknown type, someone else's project: refused.
        assert!(settings(State(s.clone()), Extension(user("u1")), Path("p1".into()), body(None, None, Some("m2"))).await.is_err());
        assert!(settings(State(s.clone()), Extension(user("u1")), Path("p1".into()), body(Some("movie"), None, None)).await.is_err());
        assert!(matches!(
            settings(State(s.clone()), Extension(user("u2")), Path("p1".into()), body(Some("game"), None, None)).await,
            Err(ApiError::NotFound)
        ));
        // "" clears the repo.
        settings(State(s), Extension(user("u1")), Path("p1".into()), body(None, Some(""), Some(""))).await.unwrap();
        let row: (Option<String>, Option<String>) =
            sqlx::query_as("SELECT repo_folder, repo_machine_id FROM projects WHERE id = 'p1'").fetch_one(&db).await.unwrap();
        assert_eq!(row, (None, None));
    }

    /// PROJ-ARCHIVE: archive keeps its first date, unarchive clears it, only the owner may, the list
    /// carries `archivedAt`, a running task keeps running, and every change is announced.
    #[tokio::test]
    async fn archive_and_unarchive_a_project() {
        let db = db().await;
        let s = state(db.clone());
        let mut events = s.bus.subscribe();
        sqlx::query("INSERT INTO tasks (id, project_id, title, description, state, updated_at, user_id) VALUES ('t1', 'p1', 'T', 'd', 'running', '2026', 'u1')")
            .execute(&db)
            .await
            .unwrap();
        let archive = |on: bool, who: &str| {
            settings(
                State(s.clone()),
                Extension(user(who)),
                Path("p1".into()),
                body(json!({ "archived": on })),
            )
        };
        let archived_at = || async { sqlx::query_scalar::<_, Option<String>>("SELECT archived_at FROM projects WHERE id = 'p1'").fetch_one(&db).await.unwrap() };
        archive(true, "u1").await.unwrap();
        assert!(archived_at().await.is_some(), "archiving sets the date");
        assert!(matches!(events.try_recv(), Ok((ref u, crate::events::Event::Changed { what: "projects", .. })) if u == "u1"));
        // A second archive keeps the first date.
        sqlx::query("UPDATE projects SET archived_at = '2026-10-01T09:00:00Z' WHERE id = 'p1'").execute(&db).await.unwrap();
        archive(true, "u1").await.unwrap();
        assert_eq!(archived_at().await.as_deref(), Some("2026-10-01T09:00:00Z"));
        // The list carries it as archivedAt.
        let Json(list) = crate::api::projects(State(s.clone()), Extension(user("u1"))).await.unwrap();
        assert_eq!(serde_json::to_value(&list[0]).unwrap()["archivedAt"], "2026-10-01T09:00:00Z");
        // Someone else's project: not found, nothing changes.
        assert!(matches!(archive(false, "u2").await, Err(ApiError::NotFound)));
        assert!(archived_at().await.is_some());
        // Archiving never stops a running task.
        let st: String = sqlx::query_scalar("SELECT state FROM tasks WHERE id = 't1'").fetch_one(&db).await.unwrap();
        assert_eq!(st, "running");
        // Unarchive clears it.
        archive(false, "u1").await.unwrap();
        assert_eq!(archived_at().await, None);
        let Json(list) = crate::api::projects(State(s), Extension(user("u1"))).await.unwrap();
        assert!(serde_json::to_value(&list[0]).unwrap()["archivedAt"].is_null());
    }

    #[tokio::test]
    async fn assets_attach_list_detach_and_cascade() {
        let db = db().await;
        let s = state(db.clone());
        let attach_it = |a: i64| attach(State(s.clone()), Extension(user("u1")), Path("p1".into()), Json(AttachBody { asset_id: a }));
        attach_it(7).await.unwrap();
        attach_it(7).await.unwrap(); // twice is fine
        assert!(matches!(attach_it(99).await, Err(ApiError::NotFound)));
        let Json(list) = assets(State(s.clone()), Extension(user("u1")), Path("p1".into())).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!((list[0]["name"].as_str(), list[0]["pack"].as_str()), (Some("tree.png"), Some("Forest pack")));
        assert!(list[0]["preview"].is_null());
        // Only the owner sees them.
        assert!(assets(State(s.clone()), Extension(user("u2")), Path("p1".into())).await.is_err());
        // Deleting the asset from the library removes the link.
        sqlx::query("DELETE FROM asset WHERE id = 7").execute(&db).await.unwrap();
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM project_asset").fetch_one(&db).await.unwrap();
        assert_eq!(n, 0);
        attach_it(7).await.unwrap_err();
        // Detach.
        sqlx::query("INSERT INTO asset (id, pack_id, path, name, ext, size, category, rule) VALUES (8, 1, 'a.ogg', 'a.ogg', 'ogg', 1, 'sound', 'ext')")
            .execute(&db)
            .await
            .unwrap();
        attach_it(8).await.unwrap();
        detach(State(s.clone()), Extension(user("u1")), Path(("p1".into(), 8))).await.unwrap();
        let Json(list) = assets(State(s), Extension(user("u1")), Path("p1".into())).await.unwrap();
        assert!(list.is_empty());
    }
}
