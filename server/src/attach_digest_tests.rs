use super::*;
use axum::{Json, Router, http::StatusCode, routing::post};
use std::{io::Cursor, path::{Path, PathBuf}, sync::{Arc, Mutex}};

/// What the mock model got: (system, user) of each call, and how it answers.
#[derive(Clone, Default)]
struct Mock {
    seen: Arc<Mutex<Vec<(String, String)>>>,
    /// The call (1-based) that is refused with this text (a 400).
    refuse_at: Option<(usize, &'static str)>,
    /// Characters added to every part digest, to make them long.
    pad: usize,
}

async fn model(m: Mock) -> String {
    let app = Router::new().route("/v1/chat/completions", post(move |Json(b): Json<Value>| {
        let m = m.clone();
        async move {
            let (system, user) = (b["messages"][0]["content"].as_str().unwrap_or("").to_string(), b["messages"][1]["content"].as_str().unwrap_or("").to_string());
            let n = { let mut seen = m.seen.lock().unwrap(); seen.push((system.clone(), user)); seen.len() };
            if let Some((at, text)) = m.refuse_at.filter(|(at, _)| *at == n) {
                let _ = at;
                return (StatusCode::BAD_REQUEST, Json(json!({ "error": { "message": text } })));
            }
            let content = if system.contains("Merge them") { "- merged digest".to_string() } else { format!("- digest of call {n}{}", "x".repeat(m.pad)) };
            (StatusCode::OK, Json(json!({ "choices": [{ "message": { "role": "assistant", "content": content }, "finish_reason": "stop" }], "usage": { "prompt_tokens": 100, "completion_tokens": 20 } })))
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/v1")
}

async fn state(url: &str) -> AppState {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026')").execute(&db).await.unwrap();
    sqlx::query("INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1', NULL, 'Chat', '2026', 'u1')").execute(&db).await.unwrap();
    let cfg = format!("[[provider]]\nid = \"fake\"\nname = \"Fake\"\nkind = \"openai-compatible\"\nbase_url = \"{url}\"\nlocal = true\n");
    AppState::for_tests(toml::from_str(&cfg).unwrap(), db)
}

fn digester(s: &AppState) -> Digester {
    let role = RoleAssignment { role: "orchestrator".into(), provider_id: "fake".into(), model_id: "m".into(), effort: "auto".into() };
    Digester::new(&s.provider("fake").unwrap(), &role, "u1")
}

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("kk-digest-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn doc(dir: &Path, name: &str, text: &str) -> Doc {
    let p = dir.join(format!("{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&p, text).unwrap();
    Doc { name: name.into(), kind: "text".into(), original: Some(p.clone()), text: Some(p), size: text.len() as u64 }
}

/// `n` lines of 100 characters each: "line 17 " and filler.
fn lines(n: usize) -> String {
    (1..=n).map(|i| format!("{:<99}", format!("line {i} "))).collect::<Vec<_>>().join("\n")
}

fn small() -> Budget {
    Budget::new(Some(4_096), true)
}

#[test]
fn a_file_is_split_into_parts_that_fit_one_call_and_at_most_eight() {
    let text = lines(100); // 100 lines of 99 characters
    let (parts, more) = split_parts(Cursor::new(text.clone().into_bytes()), 1, 2_000).unwrap();
    assert!(!more && parts.len() == 5 && parts.iter().all(|p| p.2.chars().count() <= 2_000), "{}", parts.len());
    assert_eq!((parts[0].0, parts.last().unwrap().1), (1, 100), "every line is in a part, in order");
    assert!(parts.windows(2).all(|w| w[1].0 == w[0].1 + 1), "parts follow each other without gaps");
    let (from, _) = split_parts(Cursor::new(text.clone().into_bytes()), 61, 2_000).unwrap();
    assert_eq!((from[0].0, from.last().unwrap().1), (61, 100));
    let (one, more) = split_parts(Cursor::new(b"only line".to_vec()), 1, 2_000).unwrap();
    assert_eq!((one.len(), more, one[0].2.as_str()), (1, false, "only line\n"));
    assert!(split_parts(Cursor::new(text.clone().into_bytes()), 500, 2_000).unwrap().0.is_empty());
    // A big file stops after eight parts and says the file goes on.
    let big = lines(400);
    let (eight, more) = split_parts(Cursor::new(big.into_bytes()), 1, 2_000).unwrap();
    assert!(more && eight.len() == MAX_PARTS, "{}", eight.len());
    // One line over the size is a part of its own, not an endless loop (scan already splits long lines).
    let (long, _) = split_parts(Cursor::new("x".repeat(5_000).into_bytes()), 1, 2_000).unwrap();
    assert!(long.len() >= 2 && long.iter().all(|p| p.2.chars().count() <= 1_001), "{}", long.len());
}

#[tokio::test]
async fn a_digest_has_one_call_per_part_with_its_lines_and_focus_and_says_the_file_ends() {
    let mock = Mock::default();
    let s = state(&model(mock.clone()).await).await;
    let d = tmp();
    let docs = vec![doc(&d, "notes.txt", &lines(100))];
    let b = small();
    let out = run(&s, &digester(&s), "c1", &docs, &json!({ "focus": "who decided what" }), 4_000, b).await.unwrap();
    let seen = mock.seen.lock().unwrap().clone();
    let parts = seen.len();
    assert!(parts >= 2, "a 10 KB file is more than one part of a small model");
    assert!(seen.iter().all(|(sys, user)| sys.contains("at most 8") && user.starts_with("File: notes.txt, lines ") && user.contains("The reader cares most about: who decided what")), "{seen:?}");
    assert!(seen[0].1.contains("lines 1-") && seen[0].1.contains("line 1 ") && !seen[0].1.contains("line 99 "), "the first part is the start");
    assert!(out.starts_with("Digest of notes.txt, lines 1-100 (focus: who decided what):\nLines 1-"), "{out}");
    assert!(out.contains("- digest of call 1") && out.contains("- digest of call 2"), "{out}");
    assert!(out.contains(&format!("[{parts} parts digested. This is the end of the file. For the details of a part: read_attachment name=\"notes.txt\" from_line=N.]")), "{out}");
    // Each call is in the call log under its own reason, and the context meter does not count them as the chat's context.
    let logged: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM calls WHERE reason = ? AND chat_id = 'c1'").bind(DIGEST_REASON).fetch_one(&s.db).await.unwrap();
    assert_eq!(logged, parts as i64);
    let u = crate::context::usage(&s, "c1").await.unwrap();
    assert_eq!((u.used, u.model), (None, None), "digest calls are not the chat's context");
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn a_big_file_is_digested_eight_parts_at_a_time_and_the_answer_says_where_to_go_on() {
    let mock = Mock::default();
    let s = state(&model(mock.clone()).await).await;
    let d = tmp();
    let docs = vec![doc(&d, "big.txt", &lines(900))]; // about 13 parts of this size
    let out = run(&s, &digester(&s), "c1", &docs, &json!({}), 20_000, small()).await.unwrap();
    assert_eq!(mock.seen.lock().unwrap().len(), MAX_PARTS, "one use digests at most eight parts");
    let next: usize = out.split("from_line=").nth(1).unwrap().split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap();
    assert!(out.contains("8 parts digested. To digest on: summarize_attachment name=\"big.txt\" from_line=") && next > 400, "{out}");
    // Going on from there continues, and the next use can finish the file.
    let out2 = run(&s, &digester(&s), "c1", &docs, &json!({ "from_line": next }), 20_000, small()).await.unwrap();
    assert!(out2.starts_with(&format!("Digest of big.txt, lines {next}-")), "{out2}");
    assert!(out2.contains("This is the end of the file") || out2.contains("To digest on"), "{out2}");
    let past = run(&s, &digester(&s), "c1", &docs, &json!({ "from_line": 5_000 }), 20_000, small()).await.unwrap();
    assert!(past.contains("nothing from line 5000 on"), "{past}");
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn digests_too_long_to_return_are_merged_in_groups() {
    let mock = Mock { pad: 1_500, ..Default::default() };
    let s = state(&model(mock.clone()).await).await;
    let d = tmp();
    let docs = vec![doc(&d, "notes.txt", &lines(100))];
    let out = run(&s, &digester(&s), "c1", &docs, &json!({}), 1_536, small()).await.unwrap();
    let seen = mock.seen.lock().unwrap().clone();
    let merges: Vec<_> = seen.iter().filter(|(sys, _)| sys.contains("Merge them")).collect();
    assert!(!merges.is_empty() && merges.len() < seen.len(), "the long part digests were merged: {} of {} calls", merges.len(), seen.len());
    assert!(merges[0].1.starts_with("Lines 1-") && merges[0].1.contains("- digest of call 1"), "the merge sees the marked part digests");
    assert!(out.contains("- merged digest") && !out.contains(&"x".repeat(100)), "{out}");
    assert!(out.chars().count() < 1_536 + 700, "the answer stays near its room: {}", out.chars().count());
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn a_part_that_fails_keeps_what_was_digested_and_a_refusal_teaches_the_limit() {
    // The third call is refused as too long: the first two digests are returned, and where it stopped.
    let mock = Mock { refuse_at: Some((3, "This model's maximum context length is 2048 tokens.")), ..Default::default() };
    let s = state(&model(mock.clone()).await).await;
    let d = tmp();
    let docs = vec![doc(&d, "notes.txt", &lines(300))];
    let out = run(&s, &digester(&s), "c1", &docs, &json!({}), 6_000, small()).await.unwrap();
    assert!(out.contains("- digest of call 1") && out.contains("- digest of call 2") && !out.contains("call 3"), "{out}");
    assert!(out.contains("2 parts digested; stopped at lines ") && out.contains("maximum context length is 2048") && out.contains("To digest on: summarize_attachment name=\"notes.txt\" from_line="), "{out}");
    assert_eq!(crate::context_windows::window_for(&s, "fake", "m"), Some(2_048), "the limit it named is kept for the model");
    // The first call failing: nothing to return, so an error that says why.
    let mock = Mock { refuse_at: Some((1, "no thanks")), ..Default::default() };
    let s = state(&model(mock).await).await;
    let err = run(&s, &digester(&s), "c1", &docs, &json!({}), 6_000, small()).await.unwrap_err();
    assert!(err.starts_with("error: no part could be digested") && err.contains("no thanks"), "{err}");
    // Failed calls are logged too.
    let failed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM calls WHERE reason = ? AND error IS NOT NULL").bind(DIGEST_REASON).fetch_one(&s.db).await.unwrap();
    assert_eq!(failed, 1);
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn the_tool_needs_a_file_with_text_and_the_answering_model() {
    let s = state("http://127.0.0.1:9/v1").await;
    let d = tmp();
    let mut pic = doc(&d, "cat.png", "x");
    pic.kind = "image".into();
    pic.text = None;
    let err = run(&s, &digester(&s), "c1", &[pic], &json!({ "name": "cat.png" }), 4_000, small()).await.unwrap_err();
    assert!(err.contains("No attached file is called cat.png") || err.contains("None of the attached files has text"), "{err}");
    let two = vec![doc(&d, "a.txt", "one"), doc(&d, "b.txt", "two")];
    assert!(run(&s, &digester(&s), "c1", &two, &json!({}), 4_000, small()).await.unwrap_err().starts_with("Which file?"));
    // Without the answering model (a caller that cannot give it) the tool says so instead of guessing one.
    let call = attach_read::call(&s, "c1", SUMMARIZE, &json!({}), 4_000, small(), None).await;
    assert!(call.contains("no attached files") || call.contains("needs the answering model"), "{call}");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn the_tool_is_listed_with_the_other_two_and_a_part_follows_the_window() {
    assert!(attach_read::is_tool(SUMMARIZE));
    let names: Vec<_> = attach_read::tools().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    assert_eq!(names, ["read_attachment", "search_attachment", "summarize_attachment"]);
    assert_eq!(attach_read::specs().len(), 3);
    assert!(attach_read::tools().iter().all(|t| t["description"].as_str().unwrap().len() <= 300), "the chat tool list keeps 300 characters");
    assert_eq!(attach_read::specs()[2]["function"]["parameters"]["required"], json!([]));
    // A bit over half the window goes in a part: the rest is the prompt and the answer.
    assert_eq!(Budget::new(None, true).part, 13_516, "an unknown window is a small one (8192 tokens)");
    assert_eq!(Budget::new(Some(4_096), true).part, 6_758);
    assert_eq!(Budget::new(Some(1_000_000), true).part, 60_000, "capped: a huge window does not mean a huge call");
}
