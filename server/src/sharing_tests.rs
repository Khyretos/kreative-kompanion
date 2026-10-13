//! PROJECT-SHARING tests: who may see a project, the owner-only member routes, and the read wiring
//! (project list, overview, task list) for a viewer or editor.
use super::*;
use axum::extract::Query;
use sqlx::SqlitePool;

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    for q in [
        "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'kees', 'x', '2026'), ('u2', 'mara', 'x', '2026'), ('u3', 'stranger', 'x', '2026')",
        "INSERT INTO projects (id, name, description, updated_at, user_id) VALUES ('p1', 'Shop', 'Sells plants', '2026-10-03T00:00:00Z', 'u1'), ('p2', 'Mine', '', '2026-10-01T00:00:00Z', 'u2')",
        "INSERT INTO tasks (id, project_id, title, state, updated_at, user_id, kind, position) VALUES
         ('t1', 'p1', 'Cart', 'done', '2026', 'u1', 'task', 1), ('t2', 'p1', 'Checkout', 'queued', '2026', 'u1', 'task', 2),
         ('t3', 'p2', 'Mara task', 'queued', '2026', 'u2', 'task', 1)",
    ] {
        sqlx::query(q).execute(&db).await.unwrap_or_else(|e| panic!("{e}: {q}"));
    }
    db
}

fn user(id: &str) -> User {
    User { id: id.into(), name: id.into() }
}

fn state(db: SqlitePool) -> AppState {
    AppState::for_tests(toml::from_str("").unwrap(), db)
}

async fn add_as(s: &AppState, who: &str, project: &str, name: &str, role: &str) -> ApiResult<StatusCode> {
    add(State(s.clone()), Extension(user(who)), Path(project.into()), Json(serde_json::from_value(json!({ "name": name, "role": role })).unwrap())).await
}

#[tokio::test]
async fn access_is_owner_member_or_nothing() {
    let db = db().await;
    let s = state(db.clone());
    add_as(&s, "u1", "p1", "mara", "viewer").await.unwrap();
    let owner = access(&db, "u1", "p1").await.unwrap().unwrap();
    assert_eq!((owner.owner_id.as_str(), owner.role), ("u1", Role::Owner));
    let viewer = access(&db, "u2", "p1").await.unwrap().unwrap();
    assert_eq!((viewer.owner_id.as_str(), viewer.role), ("u1", Role::Viewer));
    assert!(access(&db, "u3", "p1").await.unwrap().is_none());
    assert!(access(&db, "u1", "nope").await.unwrap().is_none());
    // Upgrading the same person to editor keeps one row.
    add_as(&s, "u1", "p1", "mara", "editor").await.unwrap();
    assert_eq!(access(&db, "u2", "p1").await.unwrap().unwrap().role, Role::Editor);
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM project_members").fetch_one(&db).await.unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn only_the_owner_adds_and_bad_requests_are_refused() {
    let db = db().await;
    let s = state(db.clone());
    assert!(matches!(add_as(&s, "u3", "p1", "mara", "viewer").await, Err(ApiError::NotFound)), "a stranger sees no project");
    add_as(&s, "u1", "p1", "mara", "viewer").await.unwrap();
    assert!(matches!(add_as(&s, "u2", "p1", "stranger", "viewer").await, Err(ApiError::NotFound)), "a viewer cannot add");
    for (name, role) in [("nobody", "viewer"), ("kees", "viewer"), ("stranger", "owner"), ("stranger", "")] {
        assert!(matches!(add_as(&s, "u1", "p1", name, role).await, Err(ApiError::BadRequest(_))), "{name} {role}");
    }
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM project_members").fetch_one(&db).await.unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn members_list_and_removal() {
    let db = db().await;
    let s = state(db.clone());
    add_as(&s, "u1", "p1", "mara", "editor").await.unwrap();
    add_as(&s, "u1", "p1", "stranger", "viewer").await.unwrap();
    let Json(v) = members(State(s.clone()), Extension(user("u2")), Path("p1".into())).await.unwrap();
    assert_eq!(v["role"], "editor");
    assert_eq!(v["owner"]["name"], "kees");
    assert_eq!(v["members"].as_array().unwrap().len(), 2);
    assert!(matches!(members(State(s.clone()), Extension(user("u3")), Path("p2".into())).await, Err(ApiError::NotFound)));
    // A member cannot remove another member; they can leave; the owner removes anyone.
    let rm = |who: &str, member: &str| remove(State(s.clone()), Extension(user(who)), Path(("p1".to_string(), member.to_string())));
    assert!(matches!(rm("u2", "u3").await, Err(ApiError::NotFound)));
    rm("u2", "u2").await.unwrap();
    assert!(access(&db, "u2", "p1").await.unwrap().is_none());
    rm("u1", "u3").await.unwrap();
    assert!(access(&db, "u3", "p1").await.unwrap().is_none());
}

#[tokio::test]
async fn a_member_sees_the_project_overview_tasks_and_list() {
    let db = db().await;
    let s = state(db.clone());
    add_as(&s, "u1", "p1", "mara", "viewer").await.unwrap();
    // Overview: the member reads it, a stranger gets 404.
    let Json(o) = crate::overview::get(State(s.clone()), Extension(user("u2")), Path("p1".into())).await.unwrap();
    assert_eq!(o["name"], "Shop");
    assert!(matches!(crate::overview::get(State(s.clone()), Extension(user("u3")), Path("p1".into())).await, Err(ApiError::NotFound)));
    // Tasks of that project: the member sees the owner's two, a stranger none.
    let q = || Query(serde_json::from_value(json!({ "project": "p1" })).unwrap());
    let Json(t) = crate::tasks::list(State(s.clone()), Extension(user("u2")), q()).await.unwrap();
    assert_eq!(t.len(), 2);
    let Json(t) = crate::tasks::list(State(s.clone()), Extension(user("u3")), q()).await.unwrap();
    assert!(t.is_empty());
    // The project list: own project plus the shared one, with the role and the owner's name.
    let Json(list) = crate::api::projects(State(s.clone()), Extension(user("u2"))).await.unwrap();
    let v = serde_json::to_value(&list).unwrap();
    let find = |id: &str| v.as_array().unwrap().iter().find(|p| p["id"] == id).cloned().unwrap();
    assert_eq!((find("p2")["role"].as_str(), find("p2")["ownerName"].as_str()), (Some("owner"), Some("mara")));
    assert_eq!((find("p1")["role"].as_str(), find("p1")["ownerName"].as_str()), (Some("viewer"), Some("kees")));
    // The owner's list has no extra projects.
    let Json(list) = crate::api::projects(State(s.clone()), Extension(user("u3"))).await.unwrap();
    assert!(list.is_empty());
}
