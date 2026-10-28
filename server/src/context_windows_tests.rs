use super::*;
use axum::Router;

async fn serve(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/v1")
}

/// A server whose model list gives `m` 32768 tokens and says nothing of `quiet`.
async fn state() -> AppState {
    let url = serve(Router::new().route("/v1/models", axum::routing::get(|| async {
        Json(json!({ "data": [{ "id": "m", "max_model_len": 32768 }, { "id": "quiet" }] }))
    }))).await;
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at, is_admin) VALUES ('a','admin','x','2026',1), ('u','user','x','2026',0)").execute(&db).await.unwrap();
    let cfg = format!("[[provider]]\nid = \"fake\"\nname = \"Fake\"\nkind = \"openai-compatible\"\nbase_url = \"{url}\"\nlocal = true\n\
                       [[provider]]\nid = \"cloud\"\nname = \"Cloud\"\nkind = \"openai-compatible\"\nbase_url = \"http://127.0.0.1:9/v1\"\n");
    AppState::for_tests(toml::from_str(&cfg).unwrap(), db)
}

fn user(id: &str) -> User {
    User { id: id.into(), name: id.into() }
}

fn p(window: Option<u32>) -> ProviderConfig {
    let w = window.map(|n| format!("\ncontext_window = {n}")).unwrap_or_default();
    toml::from_str(&format!("id = \"p\"\nname = \"P\"\nkind = \"openai-compatible\"\nbase_url = \"http://x\"{w}")).unwrap()
}

fn entry(manual: Option<i64>, learned: Option<i64>, detected: Option<i64>) -> Entry {
    Entry { manual, learned, detected, ..Default::default() }
}

#[test]
fn the_size_comes_from_the_best_source_in_a_fixed_order() {
    let all = entry(Some(1_000), Some(2_000), Some(3_000));
    let r = |e: &Entry, w: Option<u32>, model: &str| resolve(Some(e), Some(&p(w)), model);
    assert_eq!(r(&all, Some(4_000), "claude-opus-5-5"), Some((1_000, Source::Manual)), "set by hand wins over everything");
    assert_eq!(r(&entry(None, Some(2_000), Some(3_000)), Some(4_000), "claude-opus-5-5"), Some((2_000, Source::Learned)), "what a refusal taught");
    assert_eq!(r(&entry(None, None, Some(3_000)), Some(4_000), "claude-opus-5-5"), Some((4_000, Source::Provider)), "the provider's own setting");
    assert_eq!(r(&entry(None, None, Some(3_000)), None, "claude-opus-5-5"), Some((3_000, Source::Detected)), "what the server said");
    assert_eq!(r(&Entry::default(), None, "claude-opus-5-5"), Some((200_000, Source::Known)), "a cloud model by its name");
    assert_eq!(r(&Entry::default(), None, "Coder"), None, "nothing known: the caller treats it as a small model");
    assert_eq!(resolve(None, None, "Coder"), None);
    assert_eq!(Source::Learned.as_str(), "learned");
}

#[tokio::test]
async fn what_is_known_survives_a_restart_and_feeds_window_for() {
    let s = state().await;
    assert_eq!(window_for(&s, "fake", "m"), None);
    let e = Entry { manual: Some(50_000), detected: Some(32_768), detected_from: Some("the server's model list".into()), detected_at: Some(1), learned: Some(40_000), learned_at: Some(2) };
    save(&s, "fake", "m", &e).await.unwrap();
    assert_eq!(window_for(&s, "fake", "m"), Some(50_000));
    s.windows.write().unwrap().clear();
    assert_eq!(window_for(&s, "fake", "m"), None, "the cache is empty until it is read back");
    load(&s).await;
    assert_eq!(entry_of(&s, "fake", "m"), Some(e));
    assert_eq!(window_for(&s, "fake", "m"), Some(50_000));
    let other = Entry { manual: None, ..entry_of(&s, "fake", "m").unwrap() };
    save(&s, "fake", "m", &other).await.unwrap();
    assert_eq!(window_for(&s, "fake", "m"), Some(40_000), "clearing the manual size goes back to automatic");
}

#[tokio::test]
async fn a_model_nobody_asked_about_is_asked_once_and_a_known_cloud_model_never() {
    let s = state().await;
    ensure(&s, "fake", "m").await;
    let e = entry_of(&s, "fake", "m").unwrap();
    assert_eq!((e.detected, e.detected_from.as_deref()), (Some(32_768), Some("the server's model list")));
    assert!(e.detected_at.is_some());
    assert_eq!(window_for(&s, "fake", "m"), Some(32_768));
    // A server that says nothing leaves nothing, and is not asked again at every message.
    ensure(&s, "fake", "quiet").await;
    let q = entry_of(&s, "fake", "quiet").unwrap();
    assert_eq!((q.detected, q.detected_at.is_some()), (None, true));
    assert_eq!(window_for(&s, "fake", "quiet"), None);
    // A cloud model with a known size is not probed (its server is not even reachable here, so a probe would only wait).
    ensure(&s, "cloud", "claude-opus-5-5").await;
    assert_eq!(entry_of(&s, "cloud", "claude-opus-5-5"), None);
    assert_eq!(window_for(&s, "cloud", "claude-opus-5-5"), Some(200_000));
    // An unknown provider is left alone.
    ensure(&s, "nope", "m").await;
    assert_eq!(entry_of(&s, "nope", "m"), None);
}

#[tokio::test]
async fn a_refusal_that_names_a_limit_is_kept_and_only_ever_lowered() {
    let s = state().await;
    let err = "Fake answered 400 (the request was refused): This model's maximum context length is 8192 tokens. However, your messages resulted in 9100 tokens.";
    let note = learn(&s, "fake", "m", err, 30_000).await.unwrap();
    assert!(note.contains("saved about 8192 tokens as the context size of m") && note.contains("Send your message again"), "{note}");
    assert_eq!(window_for(&s, "fake", "m"), Some(8_192));
    // A higher limit later does not undo it; a lower one does lower it.
    learn(&s, "fake", "m", "maximum context length is 16000 tokens", 0).await.unwrap();
    assert_eq!(window_for(&s, "fake", "m"), Some(8_192));
    learn(&s, "fake", "m", "maximum context length is 4096 tokens", 0).await.unwrap();
    assert_eq!(window_for(&s, "fake", "m"), Some(4_096));
    // An error that names no limit teaches nothing.
    assert_eq!(learn(&s, "fake", "m", "Fake answered 500 (the service had a problem)", 5_000).await, None);
    // A size set by hand stays, and the note says it is too high.
    save(&s, "fake", "big", &Entry { manual: Some(60_000), ..Default::default() }).await.unwrap();
    let note = learn(&s, "fake", "big", "maximum context length is 8192 tokens", 0).await.unwrap();
    assert!(note.contains("8192") && note.contains("set for it in Settings") && note.contains("60000"), "{note}");
    assert_eq!(window_for(&s, "fake", "big"), Some(60_000));
    // The guard's arithmetic, measured on Coder (see context_detect_tests).
    let n = learn(&s, "fake", "Coder", "Request too large for the shared GPU: about 110829 tokens, the limit is 98000.", 235_520).await.unwrap();
    assert!(n.contains("saved about 590"), "{n}");
}

#[tokio::test]
async fn an_admin_sets_clears_and_detects_a_size_and_nobody_else_can() {
    let s = state().await;
    let form = |tokens: Option<i64>| Json(SizeForm { provider_id: "fake".into(), model_id: "m".into(), tokens });
    let Json(r) = put_size(State(s.clone()), Extension(user("a")), form(Some(60_000))).await.unwrap();
    assert_eq!((r["contextTokens"].as_i64(), r["contextSource"].as_str(), r["contextManual"].as_i64()), (Some(60_000), Some("manual"), Some(60_000)));
    let Json(r) = put_size(State(s.clone()), Extension(user("a")), form(None)).await.unwrap();
    assert_eq!((r["contextTokens"].as_i64(), r["contextManual"].as_i64()), (None, None), "cleared: nothing known yet");
    // Detect asks the server now and forgets what a refusal taught.
    learn(&s, "fake", "m", "maximum context length is 4096 tokens", 0).await.unwrap();
    assert_eq!(window_for(&s, "fake", "m"), Some(4_096));
    let Json(r) = detect(State(s.clone()), Extension(user("a")), form(None)).await.unwrap();
    assert_eq!((r["found"].as_bool(), r["contextTokens"].as_i64(), r["contextSource"].as_str(), r["contextLearned"].as_i64()), (Some(true), Some(32_768), Some("detected"), None));
    assert_eq!(r["contextFrom"], "the server's model list");
    // Bad input and the wrong person.
    for bad in [Some(10), Some(10_000_001), Some(-5)] {
        assert!(matches!(put_size(State(s.clone()), Extension(user("a")), form(bad)).await, Err(ApiError::BadRequest(_))), "{bad:?}");
    }
    let no_such = Json(SizeForm { provider_id: "nope".into(), model_id: "m".into(), tokens: Some(5_000) });
    assert!(matches!(put_size(State(s.clone()), Extension(user("a")), no_such).await, Err(ApiError::NotFound)));
    assert!(put_size(State(s.clone()), Extension(user("u")), form(Some(5_000))).await.is_err(), "a user who is not an admin");
    assert!(detect(State(s.clone()), Extension(user("u")), form(None)).await.is_err());
}

#[tokio::test]
async fn the_providers_list_shows_each_models_size_and_where_it_came_from() {
    let s = state().await;
    let prov = s.provider("fake").unwrap();
    probe_if_new(&s, &prov, "m").await;
    let f = model_fields(&s, &prov, "m");
    assert_eq!((f["id"].as_str(), f["contextTokens"].as_i64(), f["contextSource"].as_str(), f["contextDetected"].as_i64()), (Some("m"), Some(32_768), Some("detected"), Some(32_768)));
    let f = model_fields(&s, &prov, "never-seen");
    assert_eq!((f["contextTokens"].is_null(), f["contextSource"].is_null()), (true, true));
    // A model that is already known is not asked again by the list.
    save(&s, "fake", "m", &Entry { manual: Some(7_000), ..entry_of(&s, "fake", "m").unwrap() }).await.unwrap();
    probe_if_new(&s, &prov, "m").await;
    assert_eq!(entry_of(&s, "fake", "m").unwrap().manual, Some(7_000));
}
