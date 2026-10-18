// JOBS-AFTER-UPDATE: a runner exits right after a self_update job, so jobs handed out after it in the same
// poll were marked sent and never ran (a grant added next to an Update never reached the PC).
use super::*;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn setup() -> SqlitePool {
    let db = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'kees', 'x', '2026')").execute(&db).await.unwrap();
    sqlx::query("INSERT INTO machines (id, user_id, name, token_hash, created_at, last_seen) VALUES ('m1', 'u1', 'pc', 'h', '2026', NULL)").execute(&db).await.unwrap();
    db
}

async fn job(db: &SqlitePool, id: &str, at: &str, tool: Value) {
    sqlx::query("INSERT INTO machine_jobs (id, machine_id, user_id, tool, state, created_at) VALUES (?, 'm1', 'u1', ?, 'queued', ?)")
        .bind(id).bind(tool.to_string()).bind(at).execute(db).await.unwrap();
}

async fn state(db: &SqlitePool, id: &str) -> String {
    sqlx::query_scalar::<_, String>("SELECT state FROM machine_jobs WHERE id = ?").bind(id).fetch_one(db).await.unwrap()
}

fn ids(jobs: &[Value]) -> Vec<String> {
    jobs.iter().map(|j| j["id"].as_str().unwrap().to_string()).collect()
}

fn grant(target: &str) -> Value {
    json!({ "tool": "add_grant", "grant": { "target": target, "rights": ["read"] } })
}

#[tokio::test]
async fn jobs_after_a_self_update_wait_for_the_next_poll() {
    let db = setup().await;
    job(&db, "a", "2026-10-18T03:30:00Z", grant("C:\\kk-runner")).await;
    job(&db, "b", "2026-10-18T03:30:01Z", json!({ "tool": "self_update" })).await;
    job(&db, "c", "2026-10-18T03:30:02Z", grant("C:\\other")).await;
    let first = take_jobs(&db, "m1").await.unwrap();
    assert_eq!(ids(&first), ["a", "b"]);
    assert_eq!(state(&db, "a").await, "sent");
    assert_eq!(state(&db, "b").await, "sent");
    assert_eq!(state(&db, "c").await, "queued", "the grant after the update stays queued for the restarted runner");
    let second = take_jobs(&db, "m1").await.unwrap();
    assert_eq!(ids(&second), ["c"]);
    assert_eq!(state(&db, "c").await, "sent");
}

#[tokio::test]
async fn a_self_update_first_goes_out_alone() {
    let db = setup().await;
    job(&db, "b", "2026-10-18T03:30:00Z", json!({ "tool": "self_update" })).await;
    job(&db, "a", "2026-10-18T03:30:01Z", grant("C:\\kk-runner")).await;
    assert_eq!(ids(&take_jobs(&db, "m1").await.unwrap()), ["b"]);
    assert_eq!(state(&db, "a").await, "queued");
}

#[tokio::test]
async fn without_an_update_all_queued_jobs_go_out_oldest_first() {
    let db = setup().await;
    job(&db, "y", "2026-10-18T03:30:01Z", grant("/b")).await;
    job(&db, "x", "2026-10-18T03:30:00Z", grant("/a")).await;
    job(&db, "z", "2026-10-18T03:30:02Z", json!({ "tool": "revoke_grant", "target": "/a" })).await;
    assert_eq!(ids(&take_jobs(&db, "m1").await.unwrap()), ["x", "y", "z"]);
}
