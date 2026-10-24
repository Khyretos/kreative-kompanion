use super::*;
use sqlx::SqlitePool;

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    let token = crate::secrets::tests::fake("ghp_");
    for q in [
        "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026'), ('u2','ana','x','2026')".to_string(),
        "INSERT INTO projects (id, name, updated_at, user_id) VALUES ('p1','Shop','2026','u1')".into(),
        "INSERT INTO tasks (id, project_id, title, state, position, updated_at, user_id) VALUES ('t1','p1','Checkout tests','running',1,'2026','u1')".into(),
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1','p1','Thread','2026','u1')".into(),
        "INSERT INTO machines (id, user_id, name, token_hash, created_at) VALUES ('m1','u1','pc','h','2026')".into(),
        "INSERT INTO runs (id, task_id, user_id, chat_id, machine_id, folder, started_at, status, step) VALUES ('r1','t1','u1','c1','m1','/srv/shop','2026-10-24T10:00:00Z','running','writing tests')".into(),
        format!("INSERT INTO calls (id, user_id, chat_id, run_id, role, provider_id, model_id, reason, request, response, tokens_in, tokens_out, ms, at)
                 VALUES ('k1','u1','c1','r1','worker','ovms','qwen','A step on a computer.','{{\"messages\":3,\"new\":[{{\"role\":\"user\",\"content\":\"push with {token}\"}}]}}','{{\"content\":\"ok\"}}',900,40,1200,'2026-10-24T10:00:01Z')"),
        "INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, result, created_at, run_id)
         VALUES ('a1','c1','u1','m1','{\"tool\":\"shell\",\"cwd\":\"/srv/shop\",\"command\":\"cargo test\"}','Run `cargo test` in /srv/shop','running',NULL,'2026-10-24T10:00:02Z','r1')".into(),
    ] {
        sqlx::query(&q).execute(&db).await.unwrap();
    }
    db
}

fn user(id: &str) -> Extension<User> {
    Extension(User { id: id.into(), name: id.into() })
}

#[tokio::test]
async fn a_run_trace_lists_calls_and_steps_in_order_masked_with_what_it_is_doing_now() {
    let s = AppState::for_tests(toml::from_str("").unwrap(), db().await);
    let Json(t) = get(State(s.clone()), user("u1"), Path("r1".into())).await.unwrap();
    assert_eq!(t["run"]["title"], "Checkout tests");
    assert_eq!(t["run"]["project"], "Shop");
    assert_eq!(t["nowDoing"], "Run `cargo test` in /srv/shop", "the running step, over the run's own step line");
    let items = t["items"].as_array().unwrap();
    assert_eq!(items.iter().map(|i| i["kind"].as_str().unwrap()).collect::<Vec<_>>(), ["call", "step"]);
    let call = &items[0];
    assert_eq!((call["model"].as_str(), call["tokensIn"].as_i64(), call["ms"].as_i64()), (Some("qwen"), Some(900), Some(1200)));
    let prompt = call["prompt"].as_str().unwrap();
    assert!(prompt.contains("[hidden secret]") && !prompt.contains("ghp_"), "{prompt}");
    assert_eq!(items[1]["tool"]["command"], "cargo test");
    // Someone else's run is not found.
    assert!(matches!(get(State(s.clone()), user("u2"), Path("r1".into())).await.unwrap_err(), ApiError::NotFound));
}

#[test]
fn a_long_body_is_kept_as_valid_json_with_a_note() {
    let small = json!({ "a": 1 });
    assert_eq!(capped(&small), small.to_string());
    let big = json!({ "text": "é".repeat(BODY_CAP) });
    let stored = capped(&big);
    let v: Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(v["cut"], true);
    assert!(stored.len() < BODY_CAP + 200);
    assert!(shown(&stored).contains("[cut: "));
    assert!(shown(&json!("x".repeat(20_000)).to_string()).contains("[shortened here"));
}

#[tokio::test]
async fn bodies_older_than_the_retention_are_emptied_and_the_numbers_stay() {
    let db = db().await;
    let parse = |t: &str| time::OffsetDateTime::parse(t, &time::format_description::well_known::Rfc3339).unwrap();
    // Within 30 days: kept.
    assert_eq!(prune(&db, parse("2026-11-20T00:00:00Z")).await, 0);
    assert_eq!(prune(&db, parse("2026-12-01T00:00:00Z")).await, 1);
    let (request, tokens): (String, i64) = sqlx::query_as("SELECT request, tokens_in FROM calls WHERE id = 'k1'").fetch_one(&db).await.unwrap();
    assert_eq!((request.as_str(), tokens), ("{}", 900));
    assert_eq!(prune(&db, parse("2026-12-01T00:00:00Z")).await, 0, "nothing left to empty");
}
