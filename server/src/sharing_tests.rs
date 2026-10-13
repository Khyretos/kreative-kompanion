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

#[tokio::test]
async fn overview_lists_role_and_members() {
    let db = db().await;
    let s = state(db.clone());
    add_as(&s, "u1", "p1", "mara", "editor").await.unwrap();
    let Json(o) = crate::overview::get(State(s.clone()), Extension(user("u1")), Path("p1".into())).await.unwrap();
    assert_eq!(o["role"], "owner");
    assert_eq!(o["members"].as_array().unwrap().len(), 1);
    assert_eq!(o["members"][0]["name"], "mara");
    assert_eq!(o["members"][0]["userId"], "u2");
    assert_eq!(o["members"][0]["role"], "editor");
    assert_eq!(o["members"][0]["you"], false);
    let Json(o) = crate::overview::get(State(s.clone()), Extension(user("u2")), Path("p1".into())).await.unwrap();
    assert_eq!(o["role"], "editor");
    assert_eq!(o["members"][0]["you"], true);
}

#[tokio::test]
async fn members_read_the_chats_of_a_shared_project() {
    let db = db().await;
    let s = state(db.clone());
    for q in [
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1', 'p1', 'Plan', '2026-10-03T00:00:00Z', 'u1'), ('c2', NULL, 'Private', '2026-10-03T00:00:00Z', 'u1')",
        "INSERT INTO messages (id, chat_id, author, text, at) VALUES ('m1', 'c1', 'user', 'hello', '2026-10-03T00:00:01Z'), ('m2', 'c2', 'user', 'secret', '2026-10-03T00:00:02Z')",
    ] {
        sqlx::query(q).execute(&db).await.unwrap_or_else(|e| panic!("{e}: {q}"));
    }
    let ids = |list: Vec<crate::api::Chat>| list.into_iter().map(|c| c.id).collect::<Vec<_>>();
    let Json(before) = crate::api::chats(State(s.clone()), Extension(user("u2"))).await.unwrap();
    assert!(before.is_empty(), "not shared yet");
    add_as(&s, "u1", "p1", "mara", "viewer").await.unwrap();
    // The member sees the project's chat, not the owner's loose chat; a stranger sees nothing.
    let Json(list) = crate::api::chats(State(s.clone()), Extension(user("u2"))).await.unwrap();
    assert_eq!(ids(list), vec!["c1".to_string()]);
    let Json(list) = crate::api::chats(State(s.clone()), Extension(user("u3"))).await.unwrap();
    assert!(list.is_empty());
    let Json(list) = crate::api::chats(State(s.clone()), Extension(user("u1"))).await.unwrap();
    assert_eq!(list.len(), 2);
    // Messages: the member reads the shared chat only.
    let Json(m) = crate::api::messages(State(s.clone()), Extension(user("u2")), Path("c1".into())).await.unwrap();
    assert_eq!(m.len(), 1);
    assert!(matches!(crate::api::messages(State(s.clone()), Extension(user("u2")), Path("c2".into())).await, Err(ApiError::NotFound)));
    assert!(matches!(crate::api::messages(State(s.clone()), Extension(user("u3")), Path("c1".into())).await, Err(ApiError::NotFound)));
    // Slice 2 is read-only: a viewer cannot post.
    let body = || Json(serde_json::from_value(json!({ "text": "hi" })).unwrap());
    assert!(matches!(crate::api::send(State(s.clone()), Extension(user("u2")), Path("c1".into()), body()).await, Err(ApiError::NotFound)));
    // After leaving, the chat is gone again.
    remove(State(s.clone()), Extension(user("u2")), Path(("p1".to_string(), "u2".to_string()))).await.unwrap();
    let Json(list) = crate::api::chats(State(s.clone()), Extension(user("u2"))).await.unwrap();
    assert!(list.is_empty());
}

// ---- Slice 3: editors change tasks and chat; viewers and strangers stay out; the owner's models and tools answer ----

fn change(v: Value) -> Json<crate::tasks::Change> {
    Json(serde_json::from_value(v).unwrap())
}

#[tokio::test]
async fn an_editor_changes_the_owners_tasks() {
    let db = db().await;
    let s = state(db.clone());
    add_as(&s, "u1", "p1", "mara", "editor").await.unwrap();
    let new = |title: &str| Json(serde_json::from_value(json!({ "projectId": "p1", "title": title, "description": "Goal: ship it.\nSteps: build, test. Done when it works." })).unwrap());
    let (code, Json(t)) = crate::tasks::create(State(s.clone()), Extension(user("u2")), new("Pay")).await.unwrap();
    assert_eq!(code, StatusCode::CREATED);
    let new_id = t["id"].as_str().unwrap().to_string();
    // The row stays the owner's: the owner sees it and it carries the owner's id.
    let (owner,): (String,) = sqlx::query_as("SELECT user_id FROM tasks WHERE id = ?").bind(&new_id).fetch_one(&db).await.unwrap();
    assert_eq!(owner, "u1");
    let q = || Query(serde_json::from_value(json!({ "project": "p1" })).unwrap());
    let Json(list) = crate::tasks::list(State(s.clone()), Extension(user("u1")), q()).await.unwrap();
    assert_eq!(list.len(), 3);
    // Edit, reorder and delete.
    let Json(t) = crate::tasks::update(State(s.clone()), Extension(user("u2")), Path("t2".into()), change(json!({ "title": "Pay now", "state": "in_review" }))).await.unwrap();
    assert_eq!((t["title"].as_str(), t["state"].as_str()), (Some("Pay now"), Some("in_review")));
    let order = Json(serde_json::from_value(json!({ "projectId": "p1", "ids": ["t2", "t1", new_id] })).unwrap());
    assert_eq!(crate::tasks::reorder(State(s.clone()), Extension(user("u2")), order).await.unwrap(), StatusCode::NO_CONTENT);
    assert_eq!(crate::tasks::delete(State(s.clone()), Extension(user("u2")), Path("t1".into())).await.unwrap(), StatusCode::NO_CONTENT);
    let Json(list) = crate::tasks::list(State(s.clone()), Extension(user("u1")), q()).await.unwrap();
    assert_eq!(list.len(), 2);
    // Another project of the editor's own still works, and a project they are not in does not.
    let own = Json(serde_json::from_value(json!({ "projectId": "p2", "title": "Mine too", "description": "Goal: ship it.\nSteps: build, test. Done when it works." })).unwrap());
    crate::tasks::create(State(s.clone()), Extension(user("u2")), own).await.unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE project_id = 'p2' AND user_id = 'u2'").fetch_one(&db).await.unwrap();
    assert_eq!(n, 2);
    let other = Json(serde_json::from_value(json!({ "projectId": "p1", "title": "No", "description": "Goal: ship it.\nSteps: build, test. Done when it works." })).unwrap());
    assert!(matches!(crate::tasks::create(State(s.clone()), Extension(user("u3")), other).await, Err(ApiError::NotFound)));
}

#[tokio::test]
async fn a_viewer_and_a_stranger_change_nothing() {
    let db = db().await;
    let s = state(db.clone());
    add_as(&s, "u1", "p1", "mara", "viewer").await.unwrap();
    for who in ["u2", "u3"] {
        let new = Json(serde_json::from_value(json!({ "projectId": "p1", "title": "No", "description": "Goal: ship it.\nSteps: build, test. Done when it works." })).unwrap());
        assert!(matches!(crate::tasks::create(State(s.clone()), Extension(user(who)), new).await, Err(ApiError::NotFound)), "create {who}");
        let upd = crate::tasks::update(State(s.clone()), Extension(user(who)), Path("t2".into()), change(json!({ "title": "No" }))).await;
        assert!(matches!(upd, Err(ApiError::NotFound)), "update {who}");
        let order = Json(serde_json::from_value(json!({ "projectId": "p1", "ids": ["t2", "t1"] })).unwrap());
        assert!(matches!(crate::tasks::reorder(State(s.clone()), Extension(user(who)), order).await, Err(ApiError::NotFound)), "reorder {who}");
        assert!(matches!(crate::tasks::delete(State(s.clone()), Extension(user(who)), Path("t1".into())).await, Err(ApiError::NotFound)), "delete {who}");
    }
    let (n, title): (i64, String) = sqlx::query_as("SELECT COUNT(*), MAX(title) FROM tasks WHERE project_id = 'p1' AND title IN ('Cart', 'Checkout')").fetch_one(&db).await.unwrap();
    assert_eq!((n, title.as_str()), (2, "Checkout"));
}

#[tokio::test]
async fn running_tasks_stays_with_the_owner() {
    let db = db().await;
    let s = state(db.clone());
    add_as(&s, "u1", "p1", "mara", "editor").await.unwrap();
    let start = Json(serde_json::from_value(json!({ "machine_id": "m1", "folder": "/tmp" })).unwrap());
    let r = crate::taskrun::start(State(s.clone()), Extension(user("u2")), Path("t2".into()), start).await;
    assert!(matches!(r, Err(ApiError::NotFound)), "an editor cannot start the owner's task");
}

#[tokio::test]
async fn an_editor_writes_in_the_chat_and_the_owner_answers_it() {
    let db = db().await;
    let s = state(db.clone());
    for q in [
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1', 'p1', 'Plan', '2026-10-03T00:00:00Z', 'u1')",
        // Only the owner has a model set; the editor has none.
        "INSERT INTO user_roles (user_id, role, provider_id, model_id) VALUES ('u1', 'orchestrator', 'nowhere', 'none')",
    ] {
        sqlx::query(q).execute(&db).await.unwrap_or_else(|e| panic!("{e}: {q}"));
    }
    add_as(&s, "u1", "p1", "mara", "editor").await.unwrap();
    let body = |v: Value| Json(serde_json::from_value(v).unwrap());
    let sent = crate::api::send(State(s.clone()), Extension(user("u2")), Path("c1".into()), body(json!({ "text": "hi from the editor" }))).await.unwrap();
    assert_eq!(sent, StatusCode::ACCEPTED);
    let Json(m) = crate::api::messages(State(s.clone()), Extension(user("u1")), Path("c1".into())).await.unwrap();
    assert!(m.iter().any(|m| m.author == "user" && m.text == "hi from the editor"), "the editor's message is in the owner's chat");
    assert!(!m.iter().any(|m| m.text.contains("No model is set")), "the owner's model answers, not the editor's missing one");
    // A computer and its tools are the owner's: an editor who names one is refused and nothing is stored.
    let user_messages = |m: &[crate::api::Message]| m.iter().filter(|m| m.author == "user").count();
    let before = user_messages(&m);
    let r = crate::api::send(State(s.clone()), Extension(user("u2")), Path("c1".into()), body(json!({ "text": "run it", "machine_id": "m1" }))).await;
    assert!(matches!(r, Err(ApiError::Forbidden(_))), "computer tools are owner-only");
    let Json(m) = crate::api::messages(State(s.clone()), Extension(user("u1")), Path("c1".into())).await.unwrap();
    assert_eq!(user_messages(&m), before, "a refused message is not stored");
}

#[tokio::test]
async fn a_viewer_and_a_stranger_cannot_write_in_the_chat() {
    let db = db().await;
    let s = state(db.clone());
    sqlx::query("INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1', 'p1', 'Plan', '2026-10-03T00:00:00Z', 'u1')").execute(&db).await.unwrap();
    add_as(&s, "u1", "p1", "mara", "viewer").await.unwrap();
    for who in ["u2", "u3"] {
        let body = Json(serde_json::from_value(json!({ "text": "hi" })).unwrap());
        assert!(matches!(crate::api::send(State(s.clone()), Extension(user(who)), Path("c1".into()), body).await, Err(ApiError::NotFound)), "{who}");
    }
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages WHERE chat_id = 'c1'").fetch_one(&db).await.unwrap();
    assert_eq!(n, 0);
}
