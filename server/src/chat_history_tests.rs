use super::*;

async fn db() -> sqlx::SqlitePool {
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    for q in [
        "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026'), ('u2','ana','x','2026')",
        "INSERT INTO projects (id, name, description, updated_at, user_id) VALUES ('p1','Website','','2026','u1')",
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1','p1','Invoice layout','2026-10-20T09:00:00Z','u1'),
            ('c2',NULL,'Holiday','2026-10-21T10:00:00Z','u1'), ('c9',NULL,'Ana private','2026-10-21T10:00:00Z','u2')",
        "INSERT INTO messages (id, chat_id, author, text, at) VALUES
            ('m1','c1','user','The invoice template needs the round logo','2026-10-20T09:00:00Z'),
            ('m2','c1','orchestrator','Done: the logo is in templates/invoice.html','2026-10-20T09:01:00Z'),
            ('m3','c2','user','Book the train to Porto','2026-10-21T10:00:00Z'),
            ('m9','c9','user','my invoice secret plans','2026-10-21T10:00:00Z')",
    ] {
        sqlx::query(q).execute(&db).await.unwrap();
    }
    db
}

#[tokio::test]
async fn search_finds_own_chats_with_lines_and_never_another_users() {
    let db = db().await;
    let r = search(&db, "u1", "c2", "invoice", 5).await.unwrap();
    assert!(r.contains("chat_id c1: \"Invoice layout\""), "{r}");
    assert!(r.contains("project \"Website\""), "{r}");
    assert!(r.contains("user: The [invoice] template"), "{r}");
    assert!(r.contains("you: Done: the logo is in templates/[invoice]"), "{r}");
    assert!(!r.contains("c9") && !r.contains("secret plans"), "{r}");
    let r = search(&db, "u1", "c2", "porto", 5).await.unwrap();
    assert!(r.contains("chat_id c2") && r.contains("(this chat)"), "{r}");
    let r = search(&db, "u1", "c2", "nothinglikethis", 5).await.unwrap();
    assert!(r.starts_with("No chat matches"), "{r}");
    assert!(search(&db, "u1", "c2", " ;; ", 5).await.unwrap().starts_with("error:"));
}

#[tokio::test]
async fn search_respects_the_chat_limit_and_marks_archived() {
    let db = db().await;
    sqlx::query("UPDATE chats SET archived = 1 WHERE id = 'c1'").execute(&db).await.unwrap();
    sqlx::query("INSERT INTO messages (id, chat_id, author, text, at) VALUES ('m4','c2','user','logo for the train ticket','2026-10-21T10:05:00Z')").execute(&db).await.unwrap();
    let r = search(&db, "u1", "x", "logo", 1).await.unwrap();
    assert_eq!(r.matches("chat_id ").count(), 1, "{r}");
    let r = search(&db, "u1", "x", "logo", 5).await.unwrap();
    assert_eq!(r.matches("chat_id ").count(), 2, "{r}");
    assert!(r.contains("\"Invoice layout\", last message 2026-10-20 09:00, project \"Website\", archived"), "{r}");
}

#[tokio::test]
async fn read_pages_oldest_first_with_a_cursor() {
    let db = db().await;
    for i in 0..5 {
        sqlx::query("INSERT INTO messages (id, chat_id, author, text, at) VALUES (?, 'c2', 'user', ?, ?)")
            .bind(format!("n{i}"))
            .bind(format!("message number {i}"))
            .bind(format!("2026-10-22T10:0{i}:00Z"))
            .execute(&db)
            .await
            .unwrap();
    }
    let r = read(&db, "u1", "c2", "c2", None, 2, 24_000).await.unwrap();
    assert!(r.contains("(this chat), 2 messages, oldest first"), "{r}");
    assert!(r.find("number 3").unwrap() < r.find("number 4").unwrap(), "{r}");
    assert!(r.contains("before n3"), "{r}");
    let r = read(&db, "u1", "c2", "c2", Some("n3"), 2, 24_000).await.unwrap();
    assert!(r.contains("number 1") && r.contains("number 2") && !r.contains("number 3"), "{r}");
    let r = read(&db, "u1", "c2", "c2", Some("n0"), 20, 24_000).await.unwrap();
    assert!(r.contains("Book the train") && r.ends_with("This is the start of the chat."), "{r}");
    let r = read(&db, "u1", "c2", "c2", Some("m3"), 20, 24_000).await.unwrap();
    assert!(r.contains("has no older messages"), "{r}");
    assert!(read(&db, "u1", "c2", "c2", Some("nope"), 20, 24_000).await.unwrap().starts_with("error:"));
}

#[tokio::test]
async fn read_refuses_another_users_chat() {
    let db = db().await;
    let r = read(&db, "u1", "c1", "c9", None, 20, 24_000).await.unwrap();
    assert!(r.starts_with("error: there is no chat c9"), "{r}");
    assert!(!r.contains("secret plans"));
}

#[tokio::test]
async fn read_cuts_long_messages_and_keeps_the_budget() {
    let db = db().await;
    let long = "word ".repeat(2_000);
    for i in 0..4 {
        sqlx::query("INSERT INTO messages (id, chat_id, author, text, at) VALUES (?, 'c2', 'orchestrator', ?, ?)")
            .bind(format!("l{i}"))
            .bind(&long)
            .bind(format!("2026-10-22T11:0{i}:00Z"))
            .execute(&db)
            .await
            .unwrap();
    }
    let r = read(&db, "u1", "c2", "c2", None, 20, 6_000).await.unwrap();
    assert!(r.contains("[cut: "), "{r}");
    assert!(r.len() < 7_500, "{}", r.len());
    assert!(r.contains("Older messages are left"), "{r}");
}

#[tokio::test]
async fn read_hides_sources_files_and_secrets() {
    let db = db().await;
    let text = format!("Use key {} here\n\n:::sources\n[{{\"n\":1}}]\n:::", crate::secrets::tests::fake("sk-"));
    sqlx::query("INSERT INTO messages (id, chat_id, author, text, at) VALUES ('s1', 'c2', 'orchestrator', ?, '2026-10-23T00:00:00Z')").bind(text).execute(&db).await.unwrap();
    let r = read(&db, "u1", "c2", "c2", None, 1, 24_000).await.unwrap();
    assert!(r.contains("Use key [hidden secret] here"), "{r}");
    assert!(!r.contains(":::sources") && !r.contains("Ab3dEf7h"), "{r}");
}

#[tokio::test]
async fn call_defaults_to_this_chat_and_clamps_the_limit() {
    let db = db().await;
    let r = call(&db, "u1", "c1", READ, &json!({"limit": 500}), 24_000).await;
    assert!(r.contains("Invoice layout") && r.contains("2 messages"), "{r}");
    let r = call(&db, "u1", "c1", SEARCH, &json!({"query": "porto", "limit": 0}), 24_000).await;
    assert!(r.contains("chat_id c2"), "{r}");
    assert!(call(&db, "u1", "c1", "drop_chats", &json!({}), 24_000).await.starts_with("error:"));
}

#[test]
fn tool_lists_match_and_have_descriptions() {
    let (t, s) = (tools(), specs());
    assert_eq!(t.len(), s.len());
    for (a, b) in t.iter().zip(&s) {
        assert_eq!(a["name"], b["function"]["name"]);
        assert!(is_tool(a["name"].as_str().unwrap()));
        assert!(!a["description"].as_str().unwrap().is_empty());
    }
}
