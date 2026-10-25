use super::*;
use sqlx::SqlitePool;

fn row(rowid: i64, len: usize) -> Row {
    (rowid, "user".into(), "x".repeat(len), format!("2026-10-24T10:{:02}:00Z", rowid % 60))
}

#[test]
fn windows_come_from_the_provider_or_known_models() {
    assert_eq!(known_window("claude-opus-5-5"), Some(200_000));
    assert_eq!(known_window("claude-opus-5-5[1m]"), Some(1_000_000));
    assert_eq!(known_window("gpt-4o-mini"), Some(128_000));
    assert_eq!(known_window("gpt-4.1"), Some(1_047_576));
    assert_eq!(known_window("deepseek-chat"), Some(128_000));
    assert_eq!(known_window("grok-4-fast"), Some(256_000));
    assert_eq!(known_window("x-ai/grok-3-mini"), Some(131_072));
    assert_eq!(known_window("o4-mini"), Some(200_000));
    assert_eq!(known_window("gemini-2.5-pro"), Some(1_048_576));
    assert_eq!(known_window("mistral-large-latest"), Some(128_000));
    assert_eq!(known_window("codestral-latest"), Some(256_000));
    assert_eq!(known_window("Qwen3.5-9B-int8-ov"), None, "a local model's window is unknown unless set");
    let p: ProviderConfig = toml::from_str("id = \"o\"\nname = \"O\"\nkind = \"openai-compatible\"\nbase_url = \"http://x\"\ncontext_window = 32768").unwrap();
    assert_eq!(window(Some(&p), "Qwen3.5-9B-int8-ov"), Some(32_768));
    assert_eq!(window(Some(&p), "claude-opus-5-5"), Some(32_768), "the provider's setting wins");
    assert_eq!(window(None, "mystery"), None);
}

#[test]
fn used_counts_the_cache_and_the_answer() {
    assert_eq!(used(Some(1000), Some(200), Some(50), Some(9000)), Some(10_250));
    assert_eq!(used(Some(1000), None, None, None), Some(1000));
    assert_eq!(used(None, Some(5), None, None), None);
}

#[test]
fn fit_takes_the_newest_that_fit_and_at_least_one() {
    let rows = vec![row(1, 10_000), row(2, 10_000), row(3, 10_000)];
    assert_eq!(fit(&rows, 24_000), 2);
    assert_eq!(fit(&rows, 5), 1);
    assert_eq!(fit(&[], 5), 0);
}

#[test]
fn plan_summarises_only_when_messages_fall_out() {
    let short: Vec<Row> = (1..=10).map(|i| row(i, 100)).collect();
    assert_eq!(plan(&short, false), Plan { keep: 10, summarise: false });
    // 6 x 6,000 chars: 4 fit the 24,000 budget, so 2 fell out; the summary then also takes the
    // older half, keeping the newest 2 (12,000 chars).
    let long: Vec<Row> = (1..=6).map(|i| row(i, 6_000)).collect();
    assert_eq!(plan(&long, false), Plan { keep: 2, summarise: true });
    // More than 40 short messages: past the message cap.
    let many: Vec<Row> = (1..=45).map(|i| row(i, 10)).collect();
    assert_eq!(plan(&many, false), Plan { keep: 20, summarise: true });
    // Near a full window, half of what fits goes into the summary too.
    assert_eq!(plan(&short, true), Plan { keep: 5, summarise: true });
    // A tiny chat near a full window has nothing worth summarising.
    let tiny: Vec<Row> = (1..=3).map(|i| row(i, 100)).collect();
    assert_eq!(plan(&tiny, true), Plan { keep: 3, summarise: false });
    // A single huge message is kept on its own; nothing to summarise before it.
    assert_eq!(plan(&[row(1, 90_000)], false), Plan { keep: 1, summarise: false });
}

#[test]
fn source_holds_the_old_summary_and_cuts_long_messages() {
    let rows = vec![
        (1, "user".to_string(), "Use the round logo".to_string(), "2026-10-24T10:00:00Z".to_string()),
        (2, "orchestrator".to_string(), format!("{}\n\n:::sources\n[]\n:::", "y".repeat(5_000)), "2026-10-24T10:01:00Z".to_string()),
    ];
    let s = source(Some("Earlier: the shop sells plants."), &rows);
    assert!(s.starts_with("Summary of the chat before these messages:\nEarlier: the shop sells plants."), "{s}");
    assert!(s.contains("[2026-10-24 10:00 User]\nUse the round logo"), "{s}");
    assert!(s.contains("Assistant]\nyyy") && s.contains("… [cut]") && !s.contains(":::sources"), "{s}");
    let huge: Vec<Row> = (1..=40).map(|i| (i, "user".into(), "z".repeat(2_900), "2026-10-24T10:00:00Z".into())).collect();
    let s = source(None, &huge);
    assert!(s.contains("older messages are left out here"), "{}", &s[..200]);
    assert!(s.len() < 64_000);
}

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    for q in [
        "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026'), ('u2','ana','x','2026')",
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1', NULL, 'Long chat', '2026', 'u1')",
    ] {
        sqlx::query(q).execute(&db).await.unwrap();
    }
    db
}

async fn add_messages(db: &SqlitePool, n: usize, len: usize) {
    for i in 0..n {
        sqlx::query("INSERT INTO messages (id, chat_id, author, text, at) VALUES (?, 'c1', ?, ?, ?)")
            .bind(format!("m{i}"))
            .bind(if i % 2 == 0 { "user" } else { "orchestrator" })
            .bind(format!("message {i} {}", "w".repeat(len)))
            .bind(format!("2026-10-24T10:{:02}:{:02}Z", i / 60, i % 60))
            .execute(db)
            .await
            .unwrap();
    }
}

/// A model server that streams `answer` (OpenAI-compatible) and reports usage; None = it answers 500.
async fn fake_model(answer: Option<&'static str>) -> String {
    use axum::{Router, routing::post};
    let app = Router::new().route("/v1/chat/completions", post(move || async move {
        match answer {
            Some(a) => {
                let body = format!(
                    "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                    json!({ "choices": [{ "delta": { "content": a } }] }),
                    json!({ "choices": [], "usage": { "prompt_tokens": 900, "completion_tokens": 60 } })
                );
                (axum::http::StatusCode::OK, [("content-type", "text/event-stream")], body)
            }
            None => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, [("content-type", "text/plain")], "boom".to_string()),
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/v1")
}

fn state(db: SqlitePool, url: &str) -> AppState {
    let cfg = format!("[[provider]]\nid = \"fake\"\nname = \"Fake\"\nkind = \"openai-compatible\"\nbase_url = \"{url}\"\nlocal = true\ncontext_window = 10000\n");
    AppState::for_tests(toml::from_str(&cfg).unwrap(), db)
}

fn role() -> RoleAssignment {
    RoleAssignment { role: "orchestrator".into(), provider_id: "fake".into(), model_id: "tiny".into(), effort: "auto".into() }
}

async fn log(s: &AppState, reason: &str, tokens_in: i64, at: &str) {
    sqlx::query("INSERT INTO calls (user_id, id, chat_id, role, provider_id, model_id, reason, request, response, tokens_in, tokens_out, ms, at)
                 VALUES ('u1', ?, 'c1', 'orchestrator', 'fake', 'tiny', ?, '{}', '', ?, 100, 1, ?)")
        .bind(util::new_id()).bind(reason).bind(tokens_in).bind(at).execute(&s.db).await.unwrap();
}

#[tokio::test]
async fn usage_reads_the_latest_chat_call_but_not_a_summary() {
    let s = state(db().await, "http://127.0.0.1:9/v1");
    let empty = usage(&s, "c1").await.unwrap();
    assert_eq!((empty.used, empty.window, empty.summary), (None, None, None));
    log(&s, "You sent a message in this chat.", 7_000, "2026-10-24T10:00:00Z").await;
    log(&s, SUMMARY_REASON, 50_000, "2026-10-24T10:01:00Z").await;
    let u = usage(&s, "c1").await.unwrap();
    assert_eq!((u.used, u.window, u.model.as_deref()), (Some(7_100), Some(10_000), Some("tiny")));
}

#[tokio::test]
async fn after_summary_reads_only_what_the_summary_does_not_cover() {
    let db = db().await;
    add_messages(&db, 5, 3).await;
    let all = after_summary(&db, "c1", None, 400).await.unwrap();
    assert_eq!(all.len(), 5);
    assert!(all[0].2.starts_with("message 0") && all[4].2.starts_with("message 4"));
    let x = Summary { text: "t".into(), upto_at: all[2].3.clone(), upto_rowid: all[2].0, messages: 3, at: "2026".into() };
    let rest = after_summary(&db, "c1", Some(&x), 400).await.unwrap();
    assert_eq!(rest.iter().map(|r| r.2.split(' ').nth(1).unwrap()).collect::<Vec<_>>(), ["3", "4"]);
    assert_eq!(after_summary(&db, "c1", None, 2).await.unwrap().len(), 2, "the newest two");
}

#[tokio::test]
async fn compact_stores_a_running_summary_and_logs_the_call() {
    let s = state(db().await, &fake_model(Some("The user wants the round logo on invoices.")).await);
    add_messages(&s.db, 6, 10).await;
    let rows = after_summary(&s.db, "c1", None, 400).await.unwrap();
    let x = compact(&s, "u1", "c1", &role(), &rows[..4]).await.unwrap();
    assert_eq!((x.text.as_str(), x.messages, x.upto_rowid), ("The user wants the round logo on invoices.", 4, rows[3].0));
    assert_eq!(summary(&s.db, "c1").await.unwrap(), Some(x.clone()));
    let left = after_summary(&s.db, "c1", Some(&x), 400).await.unwrap();
    assert_eq!(left.len(), 2);
    // A second summary adds on: it counts all six and covers up to the last one.
    let y = compact(&s, "u1", "c1", &role(), &left).await.unwrap();
    assert_eq!(y.messages, 6);
    assert!(after_summary(&s.db, "c1", Some(&y), 400).await.unwrap().is_empty());
    let (n, reason): (i64, String) = sqlx::query_as("SELECT COUNT(*), MAX(reason) FROM calls WHERE chat_id = 'c1'").fetch_one(&s.db).await.unwrap();
    assert_eq!((n, reason.as_str()), (2, SUMMARY_REASON));
    let u = usage(&s, "c1").await.unwrap();
    assert_eq!(u.used, None, "a summary is not the chat's context use");
    assert_eq!(u.summary.map(|x| x.messages), Some(6));
}

#[tokio::test]
async fn a_failed_summary_keeps_the_old_one() {
    let s = state(db().await, &fake_model(None).await);
    add_messages(&s.db, 4, 10).await;
    let rows = after_summary(&s.db, "c1", None, 400).await.unwrap();
    let e = compact(&s, "u1", "c1", &role(), &rows).await.unwrap_err();
    assert!(e.to_string().contains("500"), "{e}");
    assert_eq!(summary(&s.db, "c1").await.unwrap(), None);
    let (err,): (Option<String>,) = sqlx::query_as("SELECT error FROM calls WHERE chat_id = 'c1'").fetch_one(&s.db).await.unwrap();
    assert!(err.is_some(), "the failed call is in the log");
    assert!(compact(&s, "u1", "c1", &role(), &[]).await.is_err());
}

#[tokio::test]
async fn a_long_chat_is_summarised_before_the_answer_and_the_newest_go_word_for_word() {
    let s = state(db().await, &fake_model(Some("Summary of the start.")).await);
    // 8 messages of ~6,000 characters: only 4 fit the 24,000 budget.
    add_messages(&s.db, 8, 6_000).await;
    let (history, x) = crate::api::history_for(&s, "u1", "c1", &role(), &["u1".to_string()], "r1").await;
    let x = x.expect("a summary was made");
    assert_eq!(x.messages, 6);
    assert_eq!(history.len(), 2);
    assert!(history[0].1.starts_with("message 6") && history[1].1.starts_with("message 7"));
    // The next answer needs no new summary: nothing fell out since.
    let (history, y) = crate::api::history_for(&s, "u1", "c1", &role(), &["u1".to_string()], "r2").await;
    assert_eq!((history.len(), y.map(|y| y.messages)), (2, Some(6)));
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM calls WHERE reason = ?").bind(SUMMARY_REASON).fetch_one(&s.db).await.unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn when_the_summary_fails_the_newest_that_fit_still_go() {
    let s = state(db().await, &fake_model(None).await);
    add_messages(&s.db, 8, 6_000).await;
    let (history, x) = crate::api::history_for(&s, "u1", "c1", &role(), &["u1".to_string()], "r1").await;
    assert!(x.is_none());
    assert_eq!(history.len(), 3, "three of ~6,010 characters fit 24,000");
    assert!(history[2].1.starts_with("message 7"));
}

#[tokio::test]
async fn near_a_full_window_half_of_a_short_chat_is_summarised() {
    let s = state(db().await, &fake_model(Some("Short summary.")).await);
    add_messages(&s.db, 10, 10).await;
    log(&s, "You sent a message in this chat.", 8_500, "2026-10-24T11:00:00Z").await;
    let (history, x) = crate::api::history_for(&s, "u1", "c1", &role(), &["u1".to_string()], "r1").await;
    assert_eq!((history.len(), x.map(|x| x.messages)), (5, Some(5)));
}

fn user(id: &str) -> User {
    User { id: id.into(), name: id.into() }
}

#[tokio::test]
async fn the_routes_check_access_and_refuse_an_empty_summary() {
    let s = state(db().await, &fake_model(Some("Done.")).await);
    add_messages(&s.db, 2, 10).await;
    assert!(get(State(s.clone()), Extension(user("u2")), Path("c1".into())).await.is_err(), "a stranger sees nothing");
    let Json(u) = get(State(s.clone()), Extension(user("u1")), Path("c1".into())).await.unwrap();
    assert_eq!(u.chat_id, "c1");
    // No orchestrator role yet, then too little to summarise.
    assert!(compact_now(State(s.clone()), Extension(user("u1")), Path("c1".into())).await.is_err());
    sqlx::query("INSERT INTO user_roles (user_id, role, provider_id, model_id) VALUES ('u1', 'orchestrator', 'fake', 'tiny')").execute(&s.db).await.unwrap();
    let e = compact_now(State(s.clone()), Extension(user("u1")), Path("c1".into())).await.unwrap_err();
    assert!(format!("{e:?}").contains("nothing new to summarise"), "{e:?}");
    add_messages_more(&s.db).await;
    let Json(u) = compact_now(State(s.clone()), Extension(user("u1")), Path("c1".into())).await.unwrap();
    assert_eq!(u.summary.map(|x| (x.messages, x.text)), Some((3, "Done.".to_string())));
    assert!(compact_now(State(s.clone()), Extension(user("u2")), Path("c1".into())).await.is_err());
}

async fn add_messages_more(db: &SqlitePool) {
    for i in 2..5 {
        sqlx::query("INSERT INTO messages (id, chat_id, author, text, at) VALUES (?, 'c1', 'user', 'more', ?)")
            .bind(format!("x{i}"))
            .bind(format!("2026-10-24T12:00:0{i}Z"))
            .execute(db)
            .await
            .unwrap();
    }
}
