//! KG-01 tests: kinds, parents, dependencies and the task API with a tree.
use super::*;
use crate::{AppState, auth::User};
use axum::{Extension, Json, extract::{Path, Query, State}};
use serde_json::json;

fn e(a: &str, b: &str) -> (String, String) {
    (a.into(), b.into())
}

#[test]
fn kinds_are_checked() {
    for k in ["goal", "milestone", "task"] {
        assert!(check_kind(k).is_ok());
    }
    assert!(check_kind("epic").is_err());
}

#[test]
fn which_parent_a_kind_may_have() {
    assert!(!parent_allowed("goal", "goal"));
    assert!(!parent_allowed("goal", "milestone"));
    assert!(parent_allowed("milestone", "goal"));
    assert!(!parent_allowed("milestone", "milestone"));
    assert!(!parent_allowed("milestone", "task"));
    assert!(parent_allowed("task", "goal"));
    assert!(parent_allowed("task", "milestone"));
    assert!(parent_allowed("task", "task"));
}

#[test]
fn dependency_loops_are_found() {
    let edges = vec![e("a", "b"), e("b", "c")];
    assert!(dep_cycle(&edges, "c", &["a".into()]));
    assert!(dep_cycle(&edges, "a", &["a".into()]));
    assert!(!dep_cycle(&edges, "c", &["d".into()]));
    assert!(!dep_cycle(&edges, "a", &["c".into()]));
    // The task's own old edges are replaced, so they never count.
    assert!(!dep_cycle(&[e("a", "b"), e("b", "a")], "a", &[]));
}

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    for (u, p) in [("u1", "p1"), ("u1", "p2"), ("u2", "p3")] {
        sqlx::query("INSERT OR IGNORE INTO users (id, name, password_hash, created_at) VALUES (?, ?, 'x', '2026')")
            .bind(u).bind(u).execute(&db).await.unwrap();
        sqlx::query("INSERT INTO projects (id, name, updated_at, user_id) VALUES (?, ?, '2026', ?)")
            .bind(p).bind(p).bind(u).execute(&db).await.unwrap();
    }
    // g (goal) > m (milestone) > t1, t2 (tasks); x in another project; y of another user.
    for (id, p, u, kind, parent) in [
        ("g", "p1", "u1", "goal", None),
        ("m", "p1", "u1", "milestone", Some("g")),
        ("t1", "p1", "u1", "task", Some("m")),
        ("t2", "p1", "u1", "task", Some("m")),
        ("x", "p2", "u1", "task", None),
        ("y", "p3", "u2", "task", None),
    ] {
        sqlx::query("INSERT INTO tasks (id, project_id, title, updated_at, user_id, kind, parent_id) VALUES (?, ?, ?, '2026', ?, ?, ?)")
            .bind(id).bind(p).bind(id).bind(u).bind(kind).bind(parent).execute(&db).await.unwrap();
    }
    db
}

#[tokio::test]
async fn a_parent_must_fit() {
    let db = db().await;
    // Fine: a new task under the milestone, t2 under t1.
    assert!(check_parent(&db, "u1", None, "p1", "task", "m").await.is_ok());
    assert!(check_parent(&db, "u1", Some("t2"), "p1", "task", "t1").await.is_ok());
    // Another project, another user's task, a missing task: refused.
    assert!(check_parent(&db, "u1", None, "p1", "task", "x").await.is_err());
    assert!(check_parent(&db, "u1", None, "p3", "task", "y").await.is_err());
    assert!(check_parent(&db, "u1", None, "p1", "task", "nope").await.is_err());
    // Itself or its own descendant: refused (the goal can't move under its task).
    assert!(check_parent(&db, "u1", Some("t1"), "p1", "task", "t1").await.is_err());
    assert!(check_parent(&db, "u1", Some("m"), "p1", "milestone", "t1").await.is_err());
    // Kind rules: a milestone only under a goal.
    assert!(check_parent(&db, "u1", None, "p1", "milestone", "m").await.is_err());
}

#[tokio::test]
async fn dependencies_are_replaced_and_checked() {
    let db = db().await;
    assert_eq!(set_deps(&db, "u1", "t2", &["t1".into(), "t1".into()]).await.unwrap(), vec!["t1".to_string()]);
    // A loop, another project, another user, itself: refused, the old list stays.
    assert!(set_deps(&db, "u1", "t1", &["t2".into()]).await.is_err());
    assert!(set_deps(&db, "u1", "t2", &["x".into()]).await.is_err());
    assert!(set_deps(&db, "u1", "t2", &["y".into()]).await.is_err());
    assert!(set_deps(&db, "u1", "t2", &["t2".into()]).await.is_err());
    let all = deps_of(&db, "u1", Some("p1")).await.unwrap();
    assert_eq!(all.get("t2"), Some(&vec!["t1".to_string()]));
    assert_eq!(all.get("t1"), None);
    // An empty list clears them.
    assert!(set_deps(&db, "u1", "t2", &[]).await.unwrap().is_empty());
    assert!(deps_of(&db, "u1", Some("p1")).await.unwrap().is_empty());
}

#[tokio::test]
async fn deleting_a_milestone_keeps_its_tasks_under_the_goal() {
    let db = db().await;
    detach(&db, "u1", "m").await.unwrap();
    let parents: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT id, parent_id FROM tasks WHERE id IN ('t1', 't2') ORDER BY id").fetch_all(&db).await.unwrap();
    assert_eq!(parents, vec![("t1".into(), Some("g".into())), ("t2".into(), Some("g".into()))]);
}

fn state(db: &SqlitePool) -> AppState {
    let mut config: crate::config::Config = toml::from_str("").unwrap();
    config.roles.clear();
    AppState::for_tests(config, db.clone())
}

fn u1() -> User {
    User { id: "u1".into(), name: "u1".into() }
}

const DESC: &str = "**Goal:** a step.\n\n**Steps**\n1. Do it.";

/// The task API end to end: a goal with a milestone, a task with a dependency, editing, deleting.
#[tokio::test]
async fn the_task_api_stores_and_edits_a_tree() {
    let db = db().await;
    let s = state(&db);
    let new = |v: serde_json::Value| Json(serde_json::from_value::<crate::tasks::NewTask>(v).unwrap());
    let (_, Json(goal)) = crate::tasks::create(State(s.clone()), Extension(u1()),
        new(json!({ "projectId": "p2", "title": "Ship it", "description": DESC, "kind": "goal" }))).await.unwrap();
    let gid = goal["id"].as_str().unwrap().to_string();
    assert_eq!(goal["kind"], "goal");
    assert_eq!(goal["parentId"], serde_json::Value::Null);
    let (_, Json(ms)) = crate::tasks::create(State(s.clone()), Extension(u1()),
        new(json!({ "projectId": "p2", "title": "M1", "description": DESC, "kind": "milestone", "parentId": gid }))).await.unwrap();
    let mid = ms["id"].as_str().unwrap().to_string();
    assert_eq!(ms["parentId"], gid.as_str());
    let (_, Json(t)) = crate::tasks::create(State(s.clone()), Extension(u1()),
        new(json!({ "projectId": "p2", "title": "T", "description": DESC, "parentId": mid, "dependsOn": ["x"] }))).await.unwrap();
    let tid = t["id"].as_str().unwrap().to_string();
    assert_eq!(t["kind"], "task");
    assert_eq!(t["dependsOn"], json!(["x"]));
    // Bad input is refused: a milestone under a task, a parent in another project.
    assert!(crate::tasks::create(State(s.clone()), Extension(u1()),
        new(json!({ "projectId": "p2", "title": "M2", "description": DESC, "kind": "milestone", "parentId": tid }))).await.is_err());
    assert!(crate::tasks::create(State(s.clone()), Extension(u1()),
        new(json!({ "projectId": "p2", "title": "T2", "description": DESC, "parentId": "g" }))).await.is_err());
    // The list carries the tree.
    let Json(list) = crate::tasks::list(State(s.clone()), Extension(u1()),
        Query(serde_json::from_value(json!({ "project": "p2" })).unwrap())).await.unwrap();
    let row = list.iter().find(|r| r["id"] == tid.as_str()).unwrap();
    assert_eq!(row["parentId"], mid.as_str());
    assert_eq!(row["dependsOn"], json!(["x"]));
    // Edit: move the task to the top level, drop its dependency; a goal can't get a parent.
    let change = |v: serde_json::Value| Json(serde_json::from_value::<crate::tasks::Change>(v).unwrap());
    let Json(t) = crate::tasks::update(State(s.clone()), Extension(u1()), Path(tid.clone()),
        change(json!({ "parentId": "", "dependsOn": [] }))).await.unwrap();
    assert_eq!(t["parentId"], serde_json::Value::Null);
    assert_eq!(t["dependsOn"], json!([]));
    assert!(crate::tasks::update(State(s.clone()), Extension(u1()), Path(gid.clone()),
        change(json!({ "parentId": mid }))).await.is_err());
    // The goal holds a milestone, so it can't become a task.
    assert!(crate::tasks::update(State(s.clone()), Extension(u1()), Path(gid.clone()),
        change(json!({ "kind": "task" }))).await.is_err());
    // Move it back under the milestone, then delete the milestone: the task moves up to the goal.
    let _ = crate::tasks::update(State(s.clone()), Extension(u1()), Path(tid.clone()), change(json!({ "parentId": mid }))).await.unwrap();
    crate::tasks::delete(State(s.clone()), Extension(u1()), Path(mid.clone())).await.unwrap();
    let parent: Option<String> = sqlx::query_scalar("SELECT parent_id FROM tasks WHERE id = ?").bind(&tid).fetch_one(&db).await.unwrap();
    assert_eq!(parent.as_deref(), Some(gid.as_str()));
}
