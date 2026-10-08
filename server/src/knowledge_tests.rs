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
    crate::assets::ai::register_sqlite_vec();
    let db = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    ensure_vectors(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'kees', 'x', '2026-10-07')").execute(&db).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, updated_at, user_id) VALUES ('p1', 'Game', '2026', 'u1'), ('p2', 'Other', '2026', 'u1')").execute(&db).await.unwrap();
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
    let hits = search(&db, "u1", "timer node timeout", None, None, None, 5).await.unwrap();
    assert_eq!(hits[0].collection, "Godot Assistant");
    assert_eq!(hits[0].doc, "timer.md");
    assert!(hits[0].text.contains("emits timeout"));
    let only = search(&db, "u1", "timer", None, Some("Game design"), None, 5).await.unwrap();
    assert_eq!(only.len(), 1);
    assert_eq!(only[0].doc, "fun.md");
    assert!(search(&db, "someone-else", "timer", None, None, None, 5).await.unwrap().is_empty());
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
    assert_eq!(summary, "Godot Assistant: 1 added, 0 updated, 0 removed, 2 skipped");
    let again = import_openwebui(&db, path.to_str().unwrap(), "kees").await.unwrap();
    assert_eq!(again, "Godot Assistant: 0 added, 0 updated, 0 removed, 3 skipped");
    assert_eq!(search(&db, "u1", "timer timeout", None, None, None, 5).await.unwrap().len(), 1);
    assert!(import_openwebui(&db, path.to_str().unwrap(), "nobody").await.is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ranked_lists_are_fused_by_reciprocal_rank() {
    assert_eq!(rrf(&[vec![1, 2, 3], vec![3, 4]]), vec![3, 1, 2, 4]);
    assert_eq!(rrf(&[vec![5, 6], vec![]]), vec![5, 6]);
    assert_eq!(rrf(&[vec![], vec![7]]), vec![7]);
    assert!(rrf(&[]).is_empty());
}

#[tokio::test]
async fn uploaded_files_become_text() {
    assert_eq!(extract("notes.md", b"# Title\n\nBody").await.unwrap(), "# Title\n\nBody");
    assert_eq!(extract("a.TXT", b"plain").await.unwrap(), "plain");
    let html = extract("page.html", b"<html><body><h1>Timer</h1><p>Counts down.</p><script>x()</script></body></html>").await.unwrap();
    assert!(html.contains("Timer") && html.contains("Counts down.") && !html.contains("x()"));
    assert!(extract("song.mp3", b"ID3").await.unwrap_err().to_string().contains("mp3"));
}

#[test]
fn html_pages_keep_only_their_main_content() {
    let furo = "<html><head><title>Subsurf</title></head><body><nav>Menu Modeling Sculpting</nav><div><article role=\"main\" id=\"furo-main-content\"><h1>Subdivision Surface</h1><p>Splits faces.</p></article></div><footer>Previous Next</footer></body></html>";
    assert_eq!(main_content(furo), "<article role=\"main\" id=\"furo-main-content\"><h1>Subdivision Surface</h1><p>Splits faces.</p></article>");
    let plain = "<body><header>Site menu</header><MAIN class=\"x\"><p>Body text</p></MAIN><aside>Links</aside></body>";
    assert_eq!(main_content(plain), "<MAIN class=\"x\"><p>Body text</p></MAIN>");
    let whole = "<body><h1>Timer</h1></body>";
    assert_eq!(main_content(whole), whole);
    // An opening tag without its closing tag keeps the whole page.
    assert_eq!(main_content("<nav>a</nav><main><p>cut"), "<nav>a</nav><main><p>cut");
    // "<mainframe>" is not "<main>".
    assert_eq!(main_content("<mainframe>x</mainframe>"), "<mainframe>x</mainframe>");
}

#[tokio::test]
async fn html_files_are_indexed_without_their_menus() {
    let page = b"<html><body><nav>Menu Modeling Sculpting</nav><article role=\"main\"><h1>Subdivision Surface</h1></article><footer>Previous Next</footer></body></html>";
    let text = extract("modifiers/subsurf.html", page).await.unwrap();
    assert!(text.contains("Subdivision Surface") && !text.contains("Menu") && !text.contains("Previous"));
}

fn unit(i: usize) -> Vec<f32> {
    let mut v = vec![0.0f32; crate::assets::ai::DIMS];
    v[i] = 1.0;
    v
}

#[tokio::test]
async fn vectors_find_chunks_worded_differently() {
    let db = db().await;
    let c = ensure_collection(&db, "u1", "Godot", "app").await.unwrap();
    add_doc(&db, &c, "timer.md", "The Timer node emits timeout.").await.unwrap();
    add_doc(&db, &c, "signals.md", "Connect a callback to react when something happens.").await.unwrap();
    let pending = pending_chunks(&db, 10).await.unwrap();
    assert_eq!(pending.len(), 2);
    let ids: Vec<i64> = pending.iter().map(|p| p.0).collect();
    store_vectors(&db, &[(ids[0], unit(0)), (ids[1], unit(1))]).await.unwrap();
    assert!(pending_chunks(&db, 10).await.unwrap().is_empty());
    // No word in common with signals.md, but its vector is the closest.
    let hits = search(&db, "u1", "event handler", Some(&unit(1)), None, None, 5).await.unwrap();
    assert_eq!(hits[0].doc, "signals.md");
    // Words and meaning together: timer.md matches the words and is first.
    let both = search(&db, "u1", "timer", Some(&unit(0)), None, None, 5).await.unwrap();
    assert_eq!(both[0].doc, "timer.md");
    assert!(search(&db, "someone-else", "event", Some(&unit(1)), None, None, 5).await.unwrap().is_empty());
    // Deleting a document removes its vectors too.
    let (doc,): (i64,) = sqlx::query_as("SELECT id FROM knowledge_doc WHERE name = 'signals.md'").fetch_one(&db).await.unwrap();
    assert!(delete_doc(&db, "u1", doc).await.unwrap());
    assert!(!delete_doc(&db, "u1", doc).await.unwrap());
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM knowledge_vec").fetch_one(&db).await.unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn a_project_searches_its_collections_first() {
    let db = db().await;
    let game = ensure_collection(&db, "u1", "Game docs", "app").await.unwrap();
    let general = ensure_collection(&db, "u1", "General", "app").await.unwrap();
    add_doc(&db, &general, "a.md", "timer timer timer timer").await.unwrap();
    add_doc(&db, &game, "b.md", "a timer in the game").await.unwrap();
    set_projects(&db, "u1", &game, &["p1".to_string(), "nope".to_string()]).await.unwrap();
    assert_eq!(project_collections(&db, "p1").await.unwrap(), vec!["Game docs".to_string()]);
    assert!(project_collections(&db, "p2").await.unwrap().is_empty());
    let plain = search(&db, "u1", "timer", None, None, None, 5).await.unwrap();
    assert_eq!(plain[0].doc, "a.md");
    let proj = search(&db, "u1", "timer", None, None, Some("p1"), 5).await.unwrap();
    assert_eq!(proj.iter().map(|h| h.doc.as_str()).collect::<Vec<_>>(), vec!["b.md", "a.md"]);
    let listed = list(&db, "u1").await.unwrap();
    let g = listed.as_array().unwrap().iter().find(|c| c["name"] == "Game docs").unwrap();
    assert_eq!(g["projects"], serde_json::json!(["p1"]));
    assert_eq!(g["docs"], 1);
    assert_eq!(g["chunks"], 1);
    assert_eq!(g["vectors"], 0);
    assert_eq!(g["source"], "app");
    assert_eq!(g["documents"][0]["name"], "b.md");
    assert!(g["documents"][0]["id"].is_i64());
    set_projects(&db, "u1", &game, &[]).await.unwrap();
    assert!(project_collections(&db, "p1").await.unwrap().is_empty());
    assert!(delete_collection(&db, "u1", &general).await.unwrap());
    assert!(!delete_collection(&db, "someone-else", &game).await.unwrap());
    assert_eq!(list(&db, "u1").await.unwrap().as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_reimport_follows_changes_in_open_webui() {
    let db = db().await;
    let dir = std::env::temp_dir().join(format!("kk-owui-sync-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("webui.db");
    let _ = std::fs::remove_file(&path);
    let src = SqlitePoolOptions::new().max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&path).create_if_missing(true)).await.unwrap();
    for q in ["CREATE TABLE knowledge (id TEXT, name TEXT)", "CREATE TABLE knowledge_file (knowledge_id TEXT, file_id TEXT)", "CREATE TABLE file (id TEXT, filename TEXT, data TEXT)",
              "INSERT INTO knowledge VALUES ('k1', 'Godot')",
              "INSERT INTO knowledge_file VALUES ('k1', 'f1'), ('k1', 'f2')",
              r#"INSERT INTO file VALUES ('f1', 'timer.md', '{"content": "The Timer node emits timeout."}'), ('f2', 'old.md', '{"content": "Old page."}')"#] {
        sqlx::query(q).execute(&src).await.unwrap();
    }
    let p = path.to_str().unwrap();
    assert_eq!(import_openwebui(&db, p, "kees").await.unwrap(), "Godot: 2 added, 0 updated, 0 removed, 0 skipped");
    // An upload in the app's own collection with the same name is never touched by the import.
    let mine = ensure_collection(&db, "u1", "Mine", "app").await.unwrap();
    add_doc(&db, &mine, "old.md", "Mine.").await.unwrap();
    sqlx::query(r#"UPDATE file SET data = '{"content": "The Timer node emits timeout and can loop."}' WHERE id = 'f1'"#).execute(&src).await.unwrap();
    sqlx::query("DELETE FROM knowledge_file WHERE file_id = 'f2'").execute(&src).await.unwrap();
    src.close().await;
    assert_eq!(import_openwebui(&db, p, "kees").await.unwrap(), "Godot: 0 added, 1 updated, 1 removed, 0 skipped");
    assert_eq!(import_openwebui(&db, p, "kees").await.unwrap(), "Godot: 0 added, 0 updated, 0 removed, 1 skipped");
    let hits = search(&db, "u1", "loop", None, None, None, 5).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].doc, "timer.md");
    assert_eq!(search(&db, "u1", "old page mine", None, None, None, 5).await.unwrap().iter().map(|h| h.collection.as_str()).collect::<Vec<_>>(), vec!["Mine"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn open_webui_is_synced_only_when_its_file_changed() {
    let db = db().await;
    let dir = std::env::temp_dir().join(format!("kk-owui-watch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("webui.db");
    let _ = std::fs::remove_file(&path);
    let src = SqlitePoolOptions::new().max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&path).create_if_missing(true)).await.unwrap();
    for q in ["CREATE TABLE knowledge (id TEXT, name TEXT)", "CREATE TABLE knowledge_file (knowledge_id TEXT, file_id TEXT)", "CREATE TABLE file (id TEXT, filename TEXT, data TEXT)",
              "INSERT INTO knowledge VALUES ('k1', 'Godot')", "INSERT INTO knowledge_file VALUES ('k1', 'f1')",
              r#"INSERT INTO file VALUES ('f1', 'timer.md', '{"content": "The Timer node."}')"#] {
        sqlx::query(q).execute(&src).await.unwrap();
    }
    src.close().await;
    let p = path.to_str().unwrap();
    let mut stamp = None;
    assert_eq!(sync_openwebui(&db, p, "kees", &mut stamp).await.unwrap().as_deref(), Some("Godot: 1 added, 0 updated, 0 removed, 0 skipped"));
    assert!(stamp.is_some());
    assert_eq!(sync_openwebui(&db, p, "kees", &mut stamp).await.unwrap(), None);
    // A newer file is read again.
    let f = std::fs::File::options().append(true).open(&path).unwrap();
    f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5)).unwrap();
    assert_eq!(sync_openwebui(&db, p, "kees", &mut stamp).await.unwrap().as_deref(), Some("Godot: 0 added, 0 updated, 0 removed, 1 skipped"));
    assert!(sync_openwebui(&db, dir.join("missing.db").to_str().unwrap(), "kees", &mut stamp).await.is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_folder_of_documents_is_imported_once_and_kept_in_step() {
    let db = db().await;
    let dir = std::env::temp_dir().join(format!("kk-folder-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("a.md"), "Core loop\n\nA game loop repeats.").unwrap();
    std::fs::write(dir.join("sub/b.txt"), "Pacing matters.").unwrap();
    std::fs::write(dir.join("empty.md"), "  ").unwrap();
    std::fs::write(dir.join("image.png"), "not text").unwrap();
    let path = dir.to_str().unwrap();
    assert_eq!(import_folder(&db, "kees", "Game design", path).await.unwrap(), "Game design: 2 added, 0 updated, 0 removed, 1 skipped");
    assert_eq!(import_folder(&db, "kees", "Game design", path).await.unwrap(), "Game design: 0 added, 0 updated, 0 removed, 3 skipped");
    let cols = collections(&db, "u1").await.unwrap();
    assert_eq!(cols.len(), 1);
    let (source,): (String,) = sqlx::query_as("SELECT source FROM knowledge_collection WHERE user_id = 'u1'").fetch_one(&db).await.unwrap();
    assert_eq!(source, "folder");
    let names: Vec<(String,)> = sqlx::query_as("SELECT name FROM knowledge_doc ORDER BY name").fetch_all(&db).await.unwrap();
    assert_eq!(names, vec![("a.md".to_string(),), ("sub/b.txt".to_string(),)]);
    std::fs::write(dir.join("a.md"), "Core loop\n\nA changed loop.").unwrap();
    std::fs::remove_file(dir.join("sub/b.txt")).unwrap();
    assert_eq!(import_folder(&db, "kees", "Game design", path).await.unwrap(), "Game design: 0 added, 1 updated, 1 removed, 1 skipped");
    assert!(import_folder(&db, "nobody", "Game design", path).await.is_err());
    assert!(import_folder(&db, "kees", "Game design", "/nonexistent-kk").await.is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_document_can_be_read_back_and_listed_in_pages() {
    let db = db().await;
    let c = ensure_collection(&db, "u1", "Notes", "app").await.unwrap();
    let text = format!("First\n\n{}\n\n{}\n\nLast", "x".repeat(1000), "y".repeat(1000));
    let id = add_doc(&db, &c, "long.md", &text).await.unwrap();
    for i in 0..60 { add_doc(&db, &c, &format!("doc{i:02}.md"), "text").await.unwrap(); }
    let (name, read) = doc_text(&db, "u1", id).await.unwrap().unwrap();
    assert_eq!(name, "long.md");
    assert_eq!(read, text);
    assert!(doc_text(&db, "other", id).await.unwrap().is_none());
    let (page, total) = docs_page(&db, "u1", &c, "", 0).await.unwrap().unwrap();
    assert_eq!((page.len(), total), (50, 61));
    let (rest, _) = docs_page(&db, "u1", &c, "", 50).await.unwrap().unwrap();
    assert_eq!(rest.len(), 11);
    let (found, total) = docs_page(&db, "u1", &c, "doc07", 0).await.unwrap().unwrap();
    assert_eq!((found.len(), total), (1, 1));
    assert!(docs_page(&db, "other", &c, "", 0).await.unwrap().is_none());
}
