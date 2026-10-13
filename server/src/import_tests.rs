//! `import` can fill a project's About text and links (the About drafts for Kees's projects).
use super::*;
use serde_json::{Value, json};

async fn db() -> SqlitePool {
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'kees', 'x', '2026')").execute(&db).await.unwrap();
    db
}

async fn import(db: &SqlitePool, tag: &str, body: Value) -> Result<()> {
    let path = std::env::temp_dir().join(format!("kk-import-about-{}-{tag}.json", std::process::id()));
    std::fs::write(&path, body.to_string()).unwrap();
    let r = run(db, path.to_str().unwrap(), Some("kees")).await;
    let _ = std::fs::remove_file(&path);
    r
}

async fn row(db: &SqlitePool) -> (String, String, String) {
    sqlx::query_as("SELECT name, about, links FROM projects WHERE id = 'p1'").fetch_one(db).await.unwrap()
}

#[tokio::test]
async fn an_import_sets_about_and_links_and_keeps_them_when_omitted() {
    let db = db().await;
    let links = json!([{ "label": "Forgejo", "url": "https://git.example.com/x" }]);
    import(&db, "a", json!({ "projects": [{ "id": "p1", "name": "P", "about": " # Why\nBecause. ", "links": links }] })).await.unwrap();
    let (name, about, stored) = row(&db).await;
    assert_eq!((name.as_str(), about.as_str()), ("P", "# Why\nBecause."));
    assert_eq!(serde_json::from_str::<Value>(&stored).unwrap(), links);
    // A later import without about and links renames the project and keeps both.
    import(&db, "b", json!({ "projects": [{ "id": "p1", "name": "P2" }] })).await.unwrap();
    let (name, about, stored) = row(&db).await;
    assert_eq!((name.as_str(), about.as_str()), ("P2", "# Why\nBecause."));
    assert_eq!(serde_json::from_str::<Value>(&stored).unwrap(), links);
}

#[tokio::test]
async fn an_import_refuses_bad_links_and_changes_nothing() {
    let db = db().await;
    import(&db, "c", json!({ "projects": [{ "id": "p1", "name": "P", "about": "kept" }] })).await.unwrap();
    for bad in [
        json!([{ "label": "x", "url": "javascript:alert(1)" }]),
        json!((0..11).map(|i| json!({ "label": format!("l{i}"), "url": format!("https://example.com/{i}") })).collect::<Vec<_>>()),
    ] {
        assert!(import(&db, "d", json!({ "projects": [{ "id": "p1", "name": "Changed", "about": "new", "links": bad }] })).await.is_err());
    }
    let (name, about, _) = row(&db).await;
    assert_eq!((name.as_str(), about.as_str()), ("P", "kept"));
}
