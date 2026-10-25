//! RELIABILITY: runs the server was in the middle of when it stopped. At startup none of them can
//! still be going (the process that ran them is gone), so each is marked interrupted and its task
//! starts again from the folder as it is now, on the same computer with the same settings. A task
//! the server already lost once in the last hour waits for the user instead: no restart loop.
use serde_json::Value;
use sqlx::SqlitePool;

use super::StartBody;
use crate::{AppState, api, events::Event, util};

/// What happens to the task of a run the restart cut off.
#[derive(Debug, PartialEq)]
pub enum After {
    /// Started again after the runners are back.
    Again { user_id: String, task_id: String, chat_id: String, body: StartBody },
    /// Lost twice within the hour: it waits for the user.
    Held { user_id: String, chat_id: String },
}

/// Marks every run left running as interrupted and decides, per task, whether it starts again.
/// `hour_ago` is the same timestamp format as the runs' (util::now()).
pub async fn interrupted(db: &SqlitePool, now: &str, hour_ago: &str) -> Vec<After> {
    type Row = (String, String, String, String, String, String, Option<String>, String, bool, Option<String>);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, task_id, user_id, chat_id, machine_id, folder, check_cmd, protected, tests_may_change, effort
         FROM runs WHERE status = 'running'",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut out = Vec::new();
    for (run_id, task_id, user_id, chat_id, machine_id, folder, check, protected, tests_may_change, effort) in rows {
        let _ = sqlx::query("UPDATE runs SET status = 'interrupted', step = 'the server restarted', ended_at = ? WHERE id = ?")
            .bind(now)
            .bind(&run_id)
            .execute(db)
            .await;
        let (lost,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM runs WHERE task_id = ? AND status = 'interrupted' AND started_at >= ?")
            .bind(&task_id)
            .bind(hour_ago)
            .fetch_one(db)
            .await
            .unwrap_or((1,));
        if lost >= 2 {
            let _ = sqlx::query("UPDATE tasks SET state = 'needs_input', step = ?, updated_at = ? WHERE id = ? AND state = 'running'")
                .bind("the server restarted twice during this task; start it again when you are ready")
                .bind(now)
                .bind(&task_id)
                .execute(db)
                .await;
            out.push(After::Held { user_id, chat_id });
            continue;
        }
        // Queued, so starting it again is allowed; the step says why it isn't moving yet.
        let _ = sqlx::query("UPDATE tasks SET state = 'queued', step = 'starting again after a server restart', updated_at = ? WHERE id = ? AND state = 'running'")
            .bind(now)
            .bind(&task_id)
            .execute(db)
            .await;
        let protected: Vec<String> = serde_json::from_str::<Value>(&protected)
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        let body = StartBody { machine_id, folder, check: check.unwrap_or_default(), protected, tests_may_change, effort };
        out.push(After::Again { user_id, task_id, chat_id, body });
    }
    out
}

/// At startup: marks the runs, then (after the runners had time to reconnect) starts the tasks again.
pub async fn resume_after_restart(s: &AppState) {
    let hour_ago = (time::OffsetDateTime::now_utc() - time::Duration::hours(1))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let list = interrupted(&s.db, &util::now(), &hour_ago).await;
    if list.is_empty() {
        return;
    }
    tracing::info!("{} run(s) were cut off by the restart", list.len());
    let s = s.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        for after in list {
            match after {
                After::Held { user_id, chat_id } => {
                    say(&s, &user_id, &chat_id, "The server restarted during this task twice within an hour, so I stopped starting it again. Start it when you are ready.").await;
                    s.bus.send(&user_id, Event::Changed { what: "tasks", machine_id: None });
                }
                After::Again { user_id, task_id, chat_id, body } => {
                    say(&s, &user_id, &chat_id, "The server restarted while this task ran. I am starting it again from the folder as it is now.").await;
                    if let Err(e) = super::start_task(&s, &user_id, &task_id, body, false).await {
                        let why = format!("could not start again after a server restart: {e}");
                        let _ = sqlx::query("UPDATE tasks SET state = 'needs_input', step = ?, updated_at = ? WHERE id = ?")
                            .bind(&why)
                            .bind(util::now())
                            .bind(&task_id)
                            .execute(&s.db)
                            .await;
                        say(&s, &user_id, &chat_id, &format!("I {why}")).await;
                    }
                    s.bus.send(&user_id, Event::Changed { what: "tasks", machine_id: None });
                }
            }
        }
    });
}

async fn say(s: &AppState, user_id: &str, chat_id: &str, text: &str) {
    match api::insert_message(s, chat_id, "orchestrator", text).await {
        Ok(message) => s.bus.send(user_id, Event::Message { message }),
        Err(e) => tracing::warn!("restart note: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn db() -> SqlitePool {
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        for q in [
            "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026')",
            "INSERT INTO projects (id, name, updated_at, user_id) VALUES ('p1','kk-engine','2026','u1')",
            "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1','p1','Task: build','2026','u1')",
            "INSERT INTO machines (id, user_id, name, token_hash, created_at) VALUES ('m1','u1','soucouyant','h','2026')",
            "INSERT INTO tasks (id, project_id, title, state, position, updated_at, user_id) VALUES ('t1','p1','Build it','running',1,'2026','u1'), ('t2','p1','On the board','running',2,'2026','u1')",
            "INSERT INTO runs (id, task_id, user_id, chat_id, machine_id, folder, check_cmd, started_at, status, protected, tests_may_change, effort)
             VALUES ('r1','t1','u1','c1','m1','/home/kees/kk-engine','cargo test','2026-10-25T02:30:00Z','running','[\"tests/\"]',1,'high')",
        ] {
            sqlx::query(q).execute(&db).await.unwrap();
        }
        db
    }

    async fn task_state(db: &SqlitePool, id: &str) -> String {
        sqlx::query_as::<_, (String,)>("SELECT state FROM tasks WHERE id = ?").bind(id).fetch_one(db).await.unwrap().0
    }

    #[tokio::test]
    async fn a_run_cut_off_by_a_restart_is_marked_and_its_task_starts_again_with_the_same_settings() {
        let db = db().await;
        let after = interrupted(&db, "2026-10-25T03:00:00Z", "2026-10-25T02:00:00Z").await;
        assert_eq!(after, vec![After::Again {
            user_id: "u1".into(), task_id: "t1".into(), chat_id: "c1".into(),
            body: StartBody { machine_id: "m1".into(), folder: "/home/kees/kk-engine".into(), check: "cargo test".into(),
                protected: vec!["tests/".into()], tests_may_change: true, effort: Some("high".into()) },
        }]);
        let (status, ended): (String, Option<String>) = sqlx::query_as("SELECT status, ended_at FROM runs WHERE id = 'r1'").fetch_one(&db).await.unwrap();
        assert_eq!((status.as_str(), ended.as_deref()), ("interrupted", Some("2026-10-25T03:00:00Z")));
        assert_eq!(task_state(&db, "t1").await, "queued", "queued, so it may start again");
        // A board task with no run of its own is left alone.
        assert_eq!(task_state(&db, "t2").await, "running");
        // Nothing is left running: a second start finds nothing to do.
        assert!(interrupted(&db, "2026-10-25T03:01:00Z", "2026-10-25T02:01:00Z").await.is_empty());
    }

    #[tokio::test]
    async fn a_task_lost_twice_within_the_hour_waits_for_the_user() {
        let db = db().await;
        sqlx::query("INSERT INTO runs (id, task_id, user_id, chat_id, machine_id, folder, started_at, ended_at, status) VALUES ('r0','t1','u1','c1','m1','/x','2026-10-25T02:10:00Z','2026-10-25T02:20:00Z','interrupted')")
            .execute(&db).await.unwrap();
        let after = interrupted(&db, "2026-10-25T03:00:00Z", "2026-10-25T02:00:00Z").await;
        assert_eq!(after, vec![After::Held { user_id: "u1".into(), chat_id: "c1".into() }]);
        assert_eq!(task_state(&db, "t1").await, "needs_input");
    }

    #[tokio::test]
    async fn an_interruption_more_than_an_hour_ago_does_not_count() {
        let db = db().await;
        sqlx::query("INSERT INTO runs (id, task_id, user_id, chat_id, machine_id, folder, started_at, ended_at, status) VALUES ('r0','t1','u1','c1','m1','/x','2026-10-24T20:00:00Z','2026-10-24T20:20:00Z','interrupted')")
            .execute(&db).await.unwrap();
        let after = interrupted(&db, "2026-10-25T03:00:00Z", "2026-10-25T02:00:00Z").await;
        assert!(matches!(after.as_slice(), [After::Again { .. }]));
    }
}
