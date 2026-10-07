use super::*;
use sqlx::sqlite::SqlitePoolOptions;

#[test]
fn text_is_split_into_chunks_at_paragraphs() {
    let text = format!("{}\n\n{}\n\n\n{}", "a".repeat(800), "b".repeat(800), "c".repeat(3500));
    let c = chunks(&text, 1500);
    assert_eq!(c[0], "a".repeat(800));
    assert_eq!(c[1], "b".repeat(800));
    assert!(c[2..].iter().all(|x| x.len() <= 1500 && x.chars().all(|ch| ch == 'c')));
    assert_eq!(c[2..].iter().map(|x| x.len()).sum::<usize>(), 3500);
    assert!(chunks("  \n\n ", 1500).is_empty());
}

#[test]
fn search_words_become_a_safe_fts_query() {
    assert_eq!(fts_query("How do I use a Timer node?"), "\"how\" OR \"do\" OR \"use\" OR \"timer\" OR \"node\"");
    assert_eq!(fts_query("\"); DROP TABLE x; --"), "\"drop\" OR \"table\"");
    assert_eq!(fts_query("?!"), "");
}

async fn db() -> sqlx::SqlitePool {
    let db = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'kees', 'x', '2026-10-07')").execute(&db).await.unwrap();
    db
}

#[tokio::test]
async fn documents_are_found_with_their_citation() {
    let db = db().await;
    let godot = ensure_collection(&db, "u1", "Godot Assistant", "openwebui").await.unwrap();
    assert_eq!(ensure_collection(&db, "u1", "Godot Assistant", "openwebui").await.unwrap(), godot);
    let other = ensure_collection(&db, "u1", "Game design", "app").await.unwrap();
    add_doc(&db, &godot, "timer.md", "Timer\n\nThe Timer node counts down an interval and emits timeout when it reaches 0.").await.unwrap();
    add_doc(&db, &other, "fun.md", "Players enjoy a timer that creates tension.").await.unwrap();
    let hits = search(&db, "u1", "timer node timeout", None, 5).await.unwrap();
    assert_eq!(hits[0].collection, "Godot Assistant");
    assert_eq!(hits[0].doc, "timer.md");
    assert!(hits[0].text.contains("emits timeout"));
    let only = search(&db, "u1", "timer", Some("Game design"), 5).await.unwrap();
    assert_eq!(only.len(), 1);
    assert_eq!(only[0].doc, "fun.md");
    assert!(search(&db, "someone-else", "timer", None, 5).await.unwrap().is_empty());
    let text = hits_text(&hits[..1]);
    assert!(text.starts_with("[Godot Assistant / timer.md, part 1]\n"));
    let list = collections(&db, "u1").await.unwrap();
    assert_eq!(list, vec![("Game design".to_string(), 1, 1), ("Godot Assistant".to_string(), 1, 1)]);
}

#[tokio::test]
async fn open_webui_collections_are_imported_once() {
    let db = db().await;
    let dir = std::env::temp_dir().join(format!("kk-owui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("webui.db");
    let _ = std::fs::remove_file(&path);
    let src = SqlitePoolOptions::new().max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&path).create_if_missing(true)).await.unwrap();
    for q in ["CREATE TABLE knowledge (id TEXT, name TEXT)", "CREATE TABLE knowledge_file (knowledge_id TEXT, file_id TEXT)", "CREATE TABLE file (id TEXT, filename TEXT, data TEXT)",
              "INSERT INTO knowledge VALUES ('k1', 'Godot Assistant')",
              "INSERT INTO knowledge_file VALUES ('k1', 'f1'), ('k1', 'f2'), ('k1', 'f3')",
              r#"INSERT INTO file VALUES ('f1', 'timer.md', '{"content": "The Timer node emits timeout."}'), ('f2', 'empty.md', '{"content": ""}'), ('f3', 'timer.md', '{"content": "duplicate name"}')"#] {
        sqlx::query(q).execute(&src).await.unwrap();
    }
    src.close().await;
    let summary = import_openwebui(&db, path.to_str().unwrap(), "kees").await.unwrap();
    assert_eq!(summary, "Godot Assistant: 1 documents added, 2 skipped");
    let again = import_openwebui(&db, path.to_str().unwrap(), "kees").await.unwrap();
    assert_eq!(again, "Godot Assistant: 0 documents added, 3 skipped");
    assert_eq!(search(&db, "u1", "timer timeout", None, 5).await.unwrap().len(), 1);
    assert!(import_openwebui(&db, path.to_str().unwrap(), "nobody").await.is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
