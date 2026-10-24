use super::*;
use sqlx::SqlitePool;

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    for q in [
        "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026'), ('u2','ana','x','2026')",
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1', NULL, 'Chat', '2026', 'u1')",
        "INSERT INTO machines (id, user_id, name, token_hash, created_at) VALUES ('m1', 'u1', 'pc', 'h', '2026')",
        "INSERT INTO user_roles (user_id, role, provider_id, model_id) VALUES ('u1', 'orchestrator', 'fake', 'tiny')",
        r#"INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, created_at)
           VALUES ('a1', 'c1', 'u1', 'm1', '{"tool":"shell","cwd":"/srv","command":"rm -rf build"}', 'clean', 'running', '2026')"#,
    ] {
        sqlx::query(q).execute(&db).await.unwrap();
    }
    db
}

/// A model server that streams `answer` and counts its calls.
async fn fake_model(answer: &'static str, calls: std::sync::Arc<std::sync::atomic::AtomicUsize>) -> String {
    use axum::{Router, routing::post};
    let app = Router::new().route("/v1/chat/completions", post(move || async move {
        calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let body = format!("data: {}\n\ndata: [DONE]\n\n", json!({ "choices": [{ "delta": { "content": answer } }] }));
        (axum::http::StatusCode::OK, [("content-type", "text/event-stream")], body)
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/v1")
}

fn state(db: SqlitePool, url: &str) -> AppState {
    let cfg = format!("[[provider]]\nid = \"fake\"\nname = \"Fake\"\nkind = \"openai-compatible\"\nbase_url = \"{url}\"\nlocal = true\n");
    AppState::for_tests(toml::from_str(&cfg).unwrap(), db)
}

fn user(id: &str) -> User {
    User { id: id.into(), name: id.into() }
}

async fn ask(s: &AppState, who: &str, id: &str) -> ApiResult<Json<Value>> {
    explain(State(s.clone()), Extension(user(who)), Path(id.into()), Some(Json(Ask { lang: "en".into() }))).await
}

#[test]
fn commands_read_as_typed() {
    assert_eq!(command_text(&json!({ "tool": "shell", "cwd": "/srv", "command": "ls -la" })), "cd /srv && ls -la");
    assert_eq!(command_text(&json!({ "tool": "shell", "command": "ls" })), "ls");
    assert_eq!(command_text(&json!({ "tool": "package", "manager": "apt", "action": "install", "names": ["git", "curl"] })), "apt install git curl");
    assert_eq!(command_text(&json!({ "tool": "service", "action": "restart", "unit": "nginx" })), "systemctl restart nginx");
    assert_eq!(command_text(&json!({ "tool": "write_file", "path": "/a", "content": "hey" })), "write /a (3 characters)");
}

#[tokio::test]
async fn off_by_default_then_explained_once_and_cached() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let s = state(db().await, &fake_model("Deletes the **build** folder.", calls.clone()).await);
    assert_eq!(prefs(&s.db, "u1").await, Prefs::default());
    assert!(matches!(ask(&s, "u1", "a1").await.unwrap_err(), ApiError::BadRequest(_)));
    set_prefs(&s.db, "u1", Prefs { on: true, ..Prefs::default() }).await.unwrap();
    let Json(first) = ask(&s, "u1", "a1").await.unwrap();
    assert_eq!((first["text"].as_str(), first["cached"].as_bool()), (Some("Deletes the **build** folder."), Some(false)));
    let Json(again) = ask(&s, "u1", "a1").await.unwrap();
    assert_eq!(again["cached"], json!(true));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1, "the second open costs no call");
    // Verbose is another explanation; the call is logged like any other.
    set_prefs(&s.db, "u1", Prefs { on: true, verbose: true, ..Prefs::default() }).await.unwrap();
    assert_eq!(ask(&s, "u1", "a1").await.unwrap().0["cached"], json!(false));
    let logged: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM calls WHERE reason = 'explain command' AND chat_id = 'c1'").fetch_one(&s.db).await.unwrap();
    assert_eq!(logged, 2);
    // Someone else's step is not found.
    set_prefs(&s.db, "u2", Prefs { on: true, ..Prefs::default() }).await.unwrap();
    assert!(matches!(ask(&s, "u2", "a1").await.unwrap_err(), ApiError::NotFound));
}

#[tokio::test]
async fn a_picked_model_explains_and_a_half_pick_means_the_chat_model() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let s = state(db().await, &fake_model("ok", calls).await);
    set_prefs(&s.db, "u1", Prefs { on: true, verbose: false, provider_id: Some("fake".into()), model_id: Some("big".into()) }).await.unwrap();
    assert_eq!(ask(&s, "u1", "a1").await.unwrap().0["model"], json!("big"));
    set_prefs(&s.db, "u1", Prefs { on: true, verbose: false, provider_id: Some("fake".into()), model_id: Some(" ".into()) }).await.unwrap();
    assert_eq!(prefs(&s.db, "u1").await.provider_id, None);
    assert_eq!(ask(&s, "u1", "a1").await.unwrap().0["model"], json!("tiny"));
}
