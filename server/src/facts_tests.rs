use super::*;
use sqlx::SqlitePool;

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    for q in [
        "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026'), ('u2','mara','x','2026'), ('u3','stranger','x','2026')",
        "INSERT INTO projects (id, name, description, updated_at, user_id) VALUES ('p1','Shop','','2026','u1'), ('p2','Other','','2026','u1')",
        "INSERT INTO project_members (project_id, user_id, role, added_at) VALUES ('p1', 'u2', 'viewer', '2026')",
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1','p1','Deploy','2026','u1'), ('c2','p1','Design','2026','u1'), ('c3',NULL,'Loose','2026','u1'), ('c9','p2','Elsewhere','2026','u1')",
    ] {
        sqlx::query(q).execute(&db).await.unwrap_or_else(|e| panic!("{e}: {q}"));
    }
    db
}

fn state(db: SqlitePool) -> AppState {
    AppState::for_tests(toml::from_str("").unwrap(), db)
}

fn user(id: &str) -> User {
    User { id: id.into(), name: id.into() }
}

fn new_fact(scope: &str, project: Option<&str>, chat: Option<&str>, key: &str, value: &str, kind: Option<&str>) -> Json<NewFact> {
    Json(serde_json::from_value(json!({ "scope": scope, "projectId": project, "chatId": chat, "key": key, "value": value, "kind": kind })).unwrap())
}

async fn listed(s: &AppState, who: &str, chat: Option<&str>, project: Option<&str>) -> ApiResult<Value> {
    let q = serde_json::from_value(json!({ "chatId": chat, "projectId": project })).unwrap();
    get(State(s.clone()), Extension(user(who)), Query(q)).await.map(|j| j.0)
}

#[test]
fn keys_values_and_kinds_are_checked_and_values_kept_as_written() {
    assert_eq!(clean_key("  deploy   command "), Ok("deploy command".into()));
    assert!(clean_key("").is_err() && clean_key(&"k".repeat(81)).is_err());
    let odd = "C:\\Users\\kees\\My Games\n  second line: ok";
    assert_eq!(clean_value(&format!("  {odd}  ")), Ok(odd.to_string()), "trimmed only, inner text unchanged");
    assert!(clean_value("").is_err() && clean_value(&"v".repeat(2_001)).is_err());
    let e = clean_value(&format!("it is {}", crate::secrets::tests::fake("ghp_"))).unwrap_err();
    assert!(e.contains("never hold secrets"), "{e}");
    assert!(clean_value("Forgejo token: file ~/.forgejo-token").is_ok(), "where a secret lives is fine");
    assert_eq!(clean_kind(None), Ok("text".into()));
    assert_eq!(clean_kind(Some("url")), Ok("url".into()));
    assert!(clean_kind(Some("password")).is_err());
}

#[tokio::test]
async fn every_kind_of_value_round_trips_and_kind_never_changes_the_value() {
    let s = state(db().await);
    for (key, value, kind) in [
        ("notes", "Ship on Fridays only after 16:00.", "text"),
        ("repo", "/home/kees/projects/shop", "path"),
        ("docs", "https://docs.example.com/shop?x=1&y=2", "url"),
        ("office", "Calle Mayor 3, Madrid", "location"),
        ("deploy", "docker compose up -d --build", "other"),
    ] {
        let (code, Json(f)) = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p1"), None, key, value, Some(kind))).await.unwrap();
        assert_eq!((code, f.value.as_str(), f.kind.as_str(), f.source.as_str()), (StatusCode::CREATED, value, kind, "user"));
        let Json(g) = update(State(s.clone()), Extension(user("u1")), Path(f.id.clone()), Json(serde_json::from_value(json!({ "kind": "text" })).unwrap())).await.unwrap();
        assert_eq!(g.value, value);
    }
    let v = listed(&s, "u1", None, Some("p1")).await.unwrap();
    assert_eq!(v["project"].as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn facts_stay_in_their_project_and_chat() {
    let s = state(db().await);
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p1"), None, "stack", "Rust + TS", None)).await.unwrap();
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("chat", None, Some("c1"), "server", "kireserver", None)).await.unwrap();
    let v = listed(&s, "u1", Some("c1"), None).await.unwrap();
    assert_eq!((v["project"][0]["key"].as_str(), v["chat"][0]["key"].as_str()), (Some("stack"), Some("server")));
    assert_eq!((v["canEditProject"].as_bool(), v["canEditChat"].as_bool()), (Some(true), Some(true)));
    // Another chat of the same project sees the project fact, not c1's chat fact.
    let v = listed(&s, "u1", Some("c2"), None).await.unwrap();
    assert_eq!((v["project"].as_array().unwrap().len(), v["chat"].as_array().unwrap().len()), (1, 0));
    // A chat of another project sees neither.
    let v = listed(&s, "u1", Some("c9"), None).await.unwrap();
    assert_eq!((v["project"].as_array().unwrap().len(), v["chat"].as_array().unwrap().len()), (0, 0));
    // A chat outside any project has only chat facts.
    let v = listed(&s, "u1", Some("c3"), None).await.unwrap();
    assert!(v["project"].is_null() && v["chat"].as_array().unwrap().is_empty(), "{v}");
}

#[tokio::test]
async fn a_viewer_reads_but_never_writes_and_a_stranger_sees_nothing() {
    let s = state(db().await);
    let (_, Json(f)) = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p1"), None, "stack", "Rust", None)).await.unwrap();
    let v = listed(&s, "u2", None, Some("p1")).await.unwrap();
    assert_eq!((v["project"].as_array().unwrap().len(), v["canEditProject"].as_bool()), (1, Some(false)));
    let e = create(State(s.clone()), Extension(user("u2")), new_fact("project", Some("p1"), None, "x", "y", None)).await.unwrap_err();
    assert!(matches!(e, ApiError::Forbidden(_)), "{e:?}");
    assert!(matches!(delete(State(s.clone()), Extension(user("u2")), Path(f.id.clone())).await.unwrap_err(), ApiError::Forbidden(_)));
    assert!(matches!(listed(&s, "u3", None, Some("p1")).await.unwrap_err(), ApiError::NotFound));
    assert!(matches!(listed(&s, "u3", Some("c1"), None).await.unwrap_err(), ApiError::NotFound));
    assert!(matches!(delete(State(s.clone()), Extension(user("u3")), Path(f.id)).await.unwrap_err(), ApiError::NotFound));
}

#[tokio::test]
async fn one_key_per_scope_and_the_user_edits_any_fact_which_then_becomes_theirs() {
    let s = state(db().await);
    let saved = call(&s, "c1", REMEMBER, &json!({ "scope": "project", "key": "Deploy", "value": "make deploy", "kind": "other" })).await;
    assert_eq!(saved, "Saved project fact \"Deploy\".");
    let e = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p1"), None, "deploy", "x", None)).await.unwrap_err();
    assert!(format!("{e:?}").contains("already a fact named"), "{e:?}");
    // The same key in another scope is fine.
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("chat", None, Some("c1"), "deploy", "only here", None)).await.unwrap();
    let f = list(&s.db, &Scope::Project("p1".into())).await.unwrap().remove(0);
    assert_eq!(f.source, "assistant");
    let Json(g) = update(State(s.clone()), Extension(user("u1")), Path(f.id.clone()), Json(serde_json::from_value(json!({ "value": "docker compose up -d" })).unwrap())).await.unwrap();
    assert_eq!((g.value.as_str(), g.source.as_str()), ("docker compose up -d", "user"));
    // Now the model can neither change nor delete it.
    let r = call(&s, "c1", REMEMBER, &json!({ "scope": "project", "key": "deploy", "value": "rm -rf /" })).await;
    assert!(r.starts_with("error: The user set \"Deploy\""), "{r}");
    let r = call(&s, "c1", FORGET, &json!({ "scope": "project", "key": "deploy" })).await;
    assert!(r.starts_with("error: The user set"), "{r}");
    assert_eq!(list(&s.db, &Scope::Project("p1".into())).await.unwrap()[0].value, "docker compose up -d");
    // Renaming onto an existing key is refused.
    let (_, Json(other)) = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p1"), None, "stack", "Rust", None)).await.unwrap();
    assert!(update(State(s.clone()), Extension(user("u1")), Path(other.id), Json(serde_json::from_value(json!({ "key": "DEPLOY" })).unwrap())).await.is_err());
}

#[tokio::test]
async fn the_model_saves_replaces_and_forgets_its_own_facts() {
    let s = state(db().await);
    assert_eq!(call(&s, "c3", REMEMBER, &json!({ "scope": "chat", "key": "port", "value": "8095" })).await, "Saved chat fact \"port\".");
    assert_eq!(call(&s, "c3", REMEMBER, &json!({ "scope": "chat", "key": "port", "value": "8096" })).await, "Saved chat fact \"port\".");
    let all = list(&s.db, &Scope::Chat("c3".into())).await.unwrap();
    assert_eq!((all.len(), all[0].value.as_str()), (1, "8096"));
    let r = call(&s, "c3", REMEMBER, &json!({ "scope": "project", "key": "x", "value": "y" })).await;
    assert!(r.contains("not in a project"), "{r}");
    let r = call(&s, "c3", REMEMBER, &json!({ "scope": "chat", "key": "key", "value": crate::secrets::tests::fake("sk-") })).await;
    assert!(r.starts_with("error:") && r.contains("never hold secrets"), "{r}");
    assert_eq!(call(&s, "c3", FORGET, &json!({ "scope": "chat", "key": "PORT" })).await, "Deleted \"port\".");
    assert!(call(&s, "c3", FORGET, &json!({ "scope": "chat", "key": "port" })).await.starts_with("error: There is no fact"));
}

#[tokio::test]
async fn recall_finds_facts_of_the_chat_and_its_project_by_words_and_marks_them_used() {
    let s = state(db().await);
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p1"), None, "deploy command", "docker compose up -d", None)).await.unwrap();
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("chat", None, Some("c1"), "server", "kireserver", None)).await.unwrap();
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("chat", None, Some("c2"), "other chat", "not here", None)).await.unwrap();
    // Every word must match, in the name or the value, any case.
    let r = call(&s, "c1", RECALL, &json!({ "query": "Compose DEPLOY" })).await;
    assert!(r.starts_with("- deploy command: docker compose up -d") && r.ends_with("[project]") && !r.contains('\n'), "{r}");
    // No query: the project's and this chat's facts, never another chat's.
    let r = call(&s, "c1", RECALL, &json!({})).await;
    assert_eq!(r.lines().count(), 2, "{r}");
    assert!(r.contains("server: kireserver") && !r.contains("not here"), "{r}");
    assert_eq!(call(&s, "c1", RECALL, &json!({ "query": "nginx" })).await, "No fact matches \"nginx\".");
    assert_eq!(call(&s, "c3", RECALL, &json!({})).await, "No facts are saved for this chat or its project.");
    let used = list(&s.db, &Scope::Chat("c1".into())).await.unwrap();
    assert!(used[0].last_used_at.is_some());
    assert!(is_tool(RECALL) && tools().iter().any(|t| t["name"] == RECALL) && specs().iter().any(|t| t["function"]["name"] == RECALL));
}

#[tokio::test]
async fn the_prompt_block_lists_project_then_chat_facts_with_age_and_marks_them_used() {
    let s = state(db().await);
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("chat", None, Some("c1"), "server", "kireserver", None)).await.unwrap();
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p1"), None, "stack", "Rust + TS", None)).await.unwrap();
    call(&s, "c1", REMEMBER, &json!({ "scope": "project", "key": "repo", "value": "/srv/shop", "kind": "path" })).await;
    let later = time::OffsetDateTime::now_utc() + time::Duration::days(3);
    let (block, ids) = prompt_block(&s.db, "c1", later).await.unwrap();
    assert_eq!(ids.len(), 3);
    let (p, c) = (block.find("Project:").unwrap(), block.find("This chat:").unwrap());
    assert!(p < c, "{block}");
    assert!(block.contains("- stack: Rust + TS (text, set by the user, 3 d ago)"), "{block}");
    assert!(block.contains("- repo: /srv/shop (path, saved by you, 3 d ago)"), "{block}");
    assert!(block[c..].contains("- server: kireserver"), "{block}");
    let used: Vec<Option<String>> = sqlx::query_scalar("SELECT last_used_at FROM facts").fetch_all(&s.db).await.unwrap();
    assert!(used.iter().all(Option::is_some));
    assert!(prompt_block(&s.db, "c3", later).await.is_none(), "no facts, no block");
}

#[tokio::test]
async fn over_the_budget_the_least_recently_used_stay_out_but_are_kept() {
    let s = state(db().await);
    for i in 0..40 {
        let _ = create(State(s.clone()), Extension(user("u1")), new_fact("chat", None, Some("c1"), &format!("fact {i:02}"), &"v".repeat(150), None)).await.unwrap();
    }
    // fact 00 was used most recently.
    sqlx::query("UPDATE facts SET last_used_at = '2999-01-01T00:00:00Z' WHERE key = 'fact 00'").execute(&s.db).await.unwrap();
    let (block, ids) = prompt_block(&s.db, "c1", time::OffsetDateTime::now_utc()).await.unwrap();
    assert!(block.len() < 3_400, "{}", block.len());
    assert!(ids.len() < 40 && ids.len() > 10, "{}", ids.len());
    assert!(block.contains("- fact 00:"), "the most recently used is in");
    assert!(block.contains(&format!("({} older facts are not shown here.)", 40 - ids.len())), "{block}");
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM facts").fetch_one(&s.db).await.unwrap();
    assert_eq!(n, 40);
}

#[tokio::test]
async fn a_missing_path_flags_the_path_fact_and_never_deletes_it_and_free_text_is_never_flagged() {
    let s = state(db().await);
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p1"), None, "repo", "/srv/shop/", Some("path"))).await.unwrap();
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p1"), None, "note", "/srv/shop", Some("text"))).await.unwrap();
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p2"), None, "repo", "/srv/shop", Some("path"))).await.unwrap();
    flag_missing_path(&s, "c2", "/srv/shop").await;
    let p1 = list(&s.db, &Scope::Project("p1".into())).await.unwrap();
    // Flagged first in the list.
    assert_eq!((p1[0].key.as_str(), p1[0].flagged_at.is_some()), ("repo", true));
    assert!(p1[1].flagged_at.is_none(), "free text is never flagged");
    assert!(list(&s.db, &Scope::Project("p2".into())).await.unwrap()[0].flagged_at.is_none(), "another project's fact is left alone");
    let (block, _) = prompt_block(&s.db, "c2", time::OffsetDateTime::now_utc()).await.unwrap();
    assert!(block.contains("FLAGGED: may be out of date"), "{block}");
    // Editing clears the flag.
    let Json(g) = update(State(s.clone()), Extension(user("u1")), Path(p1[0].id.clone()), Json(serde_json::from_value(json!({ "value": "/srv/shop2" })).unwrap())).await.unwrap();
    assert!(g.flagged_at.is_none());
}

#[tokio::test]
async fn deleting_a_chat_or_project_takes_its_facts_along() {
    let s = state(db().await);
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("chat", None, Some("c3"), "a", "b", None)).await.unwrap();
    let _ = create(State(s.clone()), Extension(user("u1")), new_fact("project", Some("p2"), None, "a", "b", None)).await.unwrap();
    sqlx::query("DELETE FROM chats WHERE id = 'c3'").execute(&s.db).await.unwrap();
    sqlx::query("DELETE FROM chats WHERE project_id = 'p2'").execute(&s.db).await.unwrap();
    sqlx::query("DELETE FROM projects WHERE id = 'p2'").execute(&s.db).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM facts").fetch_one(&s.db).await.unwrap();
    assert_eq!(n, 0);
}

#[test]
fn tool_lists_match() {
    let (t, s) = (tools(), specs());
    assert_eq!(t.len(), s.len());
    for (a, b) in t.iter().zip(&s) {
        assert_eq!(a["name"], b["function"]["name"]);
        assert!(is_tool(a["name"].as_str().unwrap()));
    }
}
