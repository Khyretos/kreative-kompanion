use super::*;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::PathBuf;

async fn setup(tag: &str) -> (SqlitePool, PathBuf) {
    let db = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'kees', 'x', '2026'), ('u2', 'other', 'x', '2026')").execute(&db).await.unwrap();
    let root = std::env::temp_dir().join(format!("kk-stu-d1-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(&root).unwrap();
    (db, root)
}

async fn run(db: &SqlitePool, root: &Path, id: &str, user: &str, state: &str, files: &[&str]) -> Vec<PathBuf> {
    let paths: Vec<PathBuf> = files.iter().map(|f| root.join(f)).collect();
    for p in &paths { std::fs::write(p, b"png").unwrap(); }
    let outputs = serde_json::to_string(&paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()).unwrap();
    sqlx::query("INSERT INTO studio_run (id, workflow, gpu, user_id, params, models, state, outputs, started_at) VALUES (?, 'vn-portrait', 'a770', ?, '{}', '[]', ?, ?, '2026-10-08')")
        .bind(id).bind(user).bind(state).bind(outputs).execute(db).await.unwrap();
    paths
}

async fn exists(db: &SqlitePool, id: &str) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM studio_run WHERE id = ?").bind(id).fetch_one(db).await.unwrap() == 1
}

#[tokio::test]
async fn the_owner_deletes_a_finished_and_a_failed_run_with_their_files() {
    let (db, root) = setup("own").await;
    let a = run(&db, &root, "r1", "u1", "done", &["a.png", "b.png"]).await;
    let b = run(&db, &root, "r2", "u1", "failed", &["c.png"]).await;
    let r = remove_runs(&db, &root, "u1", false, &["r1".into(), "r2".into()]).await.unwrap();
    assert_eq!(r.deleted, vec!["r1".to_string(), "r2".to_string()]);
    assert!(r.skipped.is_empty());
    assert!(!exists(&db, "r1").await && !exists(&db, "r2").await);
    for p in a.iter().chain(b.iter()) { assert!(!p.exists(), "{p:?} still on disk"); }
}

#[tokio::test]
async fn another_user_cannot_delete_but_an_admin_can() {
    let (db, root) = setup("other").await;
    let a = run(&db, &root, "r1", "u1", "done", &["a.png"]).await;
    let r = remove_runs(&db, &root, "u2", false, &["r1".into()]).await.unwrap();
    assert!(r.deleted.is_empty());
    assert_eq!(r.skipped, vec!["r1".to_string()]);
    assert!(exists(&db, "r1").await && a[0].exists());
    let r = remove_runs(&db, &root, "u2", true, &["r1".into()]).await.unwrap();
    assert_eq!(r.deleted, vec!["r1".to_string()]);
    assert!(!a[0].exists());
}

#[tokio::test]
async fn a_running_or_queued_run_stays() {
    let (db, root) = setup("running").await;
    run(&db, &root, "r1", "u1", "running", &[]).await;
    run(&db, &root, "r2", "u1", "queued", &[]).await;
    let r = remove_runs(&db, &root, "u1", false, &["r1".into(), "r2".into(), "nope".into()]).await.unwrap();
    assert!(r.deleted.is_empty());
    assert_eq!(r.skipped, vec!["r1".to_string(), "r2".to_string(), "nope".to_string()]);
    assert!(exists(&db, "r1").await && exists(&db, "r2").await);
}

#[tokio::test]
async fn a_file_sent_to_assets_stays_on_disk() {
    let (db, root) = setup("assets").await;
    let a = run(&db, &root, "r1", "u1", "done", &["kept.png", "gone.png"]).await;
    let pack: i64 = sqlx::query_scalar("INSERT INTO asset_pack (key, name, kind) VALUES ('@studio', 'Kompanion Studio', 'loose') RETURNING id").fetch_one(&db).await.unwrap();
    sqlx::query("INSERT INTO asset (pack_id, container, path, name, ext, size, mtime, category, rule, provenance) VALUES (?, '', ?, 'kept.png', 'png', 3, 0, 'image', 'x', '{}')")
        .bind(pack).bind(a[0].to_string_lossy().to_string()).execute(&db).await.unwrap();
    let r = remove_runs(&db, &root, "u1", false, &["r1".into()]).await.unwrap();
    assert_eq!(r.deleted, vec!["r1".to_string()]);
    assert_eq!(r.kept_files, 1);
    assert!(a[0].exists(), "the Assets copy was removed");
    assert!(!a[1].exists());
}

#[tokio::test]
async fn a_path_outside_the_output_root_is_never_removed() {
    let (db, root) = setup("outside").await;
    let (_, other) = setup("outside-victim").await;
    let victim = other.join("victim.png");
    std::fs::write(&victim, b"x").unwrap();
    let outputs = serde_json::to_string(&vec![victim.to_string_lossy().to_string()]).unwrap();
    sqlx::query("INSERT INTO studio_run (id, workflow, gpu, user_id, params, models, state, outputs, started_at) VALUES ('r1', 'w', 'a770', 'u1', '{}', '[]', 'done', ?, '2026')")
        .bind(outputs).execute(&db).await.unwrap();
    let r = remove_runs(&db, &root, "u1", false, &["r1".into()]).await.unwrap();
    assert_eq!(r.deleted, vec!["r1".to_string()]);
    assert!(victim.exists());
}

#[tokio::test]
async fn failed_ids_lists_only_the_users_failed_runs() {
    let (db, root) = setup("failed").await;
    run(&db, &root, "r1", "u1", "failed", &[]).await;
    run(&db, &root, "r2", "u1", "done", &[]).await;
    run(&db, &root, "r3", "u2", "failed", &[]).await;
    run(&db, &root, "r4", "u1", "failed", &[]).await;
    let mut ids = failed_ids(&db, "u1").await.unwrap();
    ids.sort();
    assert_eq!(ids, vec!["r1".to_string(), "r4".to_string()]);
}
