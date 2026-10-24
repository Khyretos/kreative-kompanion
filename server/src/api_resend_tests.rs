use super::*;
use sqlx::SqlitePool;

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    for q in [
        "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026'), ('u2','ana','x','2026')",
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1', NULL, 'Chat', '2026', 'u1')",
        "INSERT INTO messages (id, chat_id, author, text, at) VALUES
           ('m1','c1','user','first','2026-10-24T10:00:00Z'), ('m2','c1','orchestrator','answer one','2026-10-24T10:00:01Z'),
           ('m3','c1','user','second\n\n:::files\n[{\"name\":\"a.txt\",\"file\":\"f1.txt\",\"kind\":\"text\"}]','2026-10-24T10:00:02Z'),
           ('m4','c1','orchestrator','answer two','2026-10-24T10:00:03Z')",
        "INSERT INTO chat_summary (chat_id, text, upto_at, upto_rowid, messages, at) VALUES ('c1', 'old', '2026-10-24T10:00:03Z', 4, 4, '2026')",
    ] {
        sqlx::query(q).execute(&db).await.unwrap();
    }
    db
}

fn user(id: &str) -> User {
    User { id: id.into(), name: id.into() }
}

async fn resend_as(s: &AppState, who: &str, mid: &str, text: Option<&str>) -> ApiResult<Json<Vec<Message>>> {
    let body = ResendBody { text: text.map(str::to_string), machine_id: None };
    resend(State(s.clone()), Extension(user(who)), Path(("c1".into(), mid.into())), Json(body)).await
}

async fn texts(db: &SqlitePool) -> Vec<String> {
    sqlx::query_scalar("SELECT text FROM messages WHERE chat_id = 'c1' ORDER BY at, rowid").fetch_all(db).await.unwrap()
}

#[tokio::test]
async fn regenerate_drops_the_answer_and_answers_the_same_message_again() {
    let s = AppState::for_tests(toml::from_str("").unwrap(), db().await);
    let Json(left) = resend_as(&s, "u1", "m3", None).await.unwrap();
    assert_eq!(left.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["m1", "m2", "m3"]);
    // No model in this config: the new answer says so, after the message.
    let now = texts(&s.db).await;
    assert_eq!(now.len(), 4);
    assert!(now[3].starts_with("No model is set"), "{now:?}");
    // The summary covered the dropped answer, so it goes too.
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_summary").fetch_one(&s.db).await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn edit_and_resend_replaces_the_text_keeps_the_files_and_drops_what_followed() {
    let s = AppState::for_tests(toml::from_str("").unwrap(), db().await);
    let Json(left) = resend_as(&s, "u1", "m1", Some("  first, but better ")).await.unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].text, "first, but better");
    resend_as(&s, "u1", "m1", Some("again")).await.unwrap();
    let s2 = AppState::for_tests(toml::from_str("").unwrap(), db().await);
    resend_as(&s2, "u1", "m3", Some("second, edited")).await.unwrap();
    assert!(texts(&s2.db).await[2].starts_with("second, edited\n\n:::files\n[{\"name\":\"a.txt\""));
}

#[tokio::test]
async fn only_the_users_own_messages_and_only_by_who_may_write() {
    let s = AppState::for_tests(toml::from_str("").unwrap(), db().await);
    assert!(matches!(resend_as(&s, "u1", "m2", None).await.unwrap_err(), ApiError::BadRequest(_)));
    assert!(matches!(resend_as(&s, "u2", "m3", None).await.unwrap_err(), ApiError::NotFound));
    assert!(matches!(resend_as(&s, "u1", "nope", None).await.unwrap_err(), ApiError::NotFound));
    assert!(matches!(resend_as(&s, "u1", "m3", Some("  ")).await.unwrap_err(), ApiError::BadRequest(_)));
    assert_eq!(texts(&s.db).await.len(), 4, "nothing changed");
}
