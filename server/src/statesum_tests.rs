//! KG-07 tests: the summary is built from rows only, says what is left, stays bounded and is rewritten on a
//! finished task.
use super::*;

fn t(id: &str, parent: &str, kind: &str, title: &str, state: &str) -> Row {
    Row { id: id.into(), parent_id: parent.into(), kind: kind.into(), title: title.into(), state: state.into() }
}

fn rows() -> Vec<Row> {
    vec![
        t("g", "", "goal", "Ship the beta", "queued"),
        t("a", "g", "task", "Login page", "done"),
        t("b", "g", "task", "Payments", "running"),
        t("c", "g", "task", "Docs", "queued"),
        t("d", "", "task", "Fix crash", "failed"),
    ]
}

#[test]
fn says_done_running_blocked_and_left() {
    let s = render("Shop", "2026-10-12 05:00", &rows(), &[("Login page".into(), "https://f/pulls/9".into())]);
    assert!(s.starts_with("State of Shop (2026-10-12 05:00 UTC)"), "{s}");
    assert!(s.contains("Tasks: 1 of 4 done"), "{s}");
    assert!(s.contains("Running: Payments"), "{s}");
    assert!(s.contains("Needs attention: Fix crash (failed)"), "{s}");
    assert!(s.contains("Left: Payments; Docs; Fix crash"), "{s}");
    assert!(s.contains("Ship the beta: 1 of 3 tasks done"), "{s}");
    assert!(s.contains("Last merged: Login page (https://f/pulls/9)"), "{s}");
}

#[test]
fn nothing_left_is_said_plainly() {
    let all = vec![t("a", "", "task", "One", "done"), t("b", "", "task", "Two", "done")];
    let s = render("P", "now", &all, &[]);
    assert!(s.contains("Tasks: 2 of 2 done"), "{s}");
    assert!(s.contains("Left: nothing"), "{s}");
    assert!(!s.contains("Last merged"), "{s}");
}

#[test]
fn empty_project_has_no_tasks_line_noise() {
    let s = render("P", "now", &[], &[]);
    assert!(s.contains("Tasks: none yet"), "{s}");
}

#[test]
fn long_projects_are_cut_to_the_limit_at_a_line_end() {
    let many: Vec<Row> = (0..400).map(|i| t(&format!("t{i}"), "", "task", &format!("Task number {i} with a long title to fill space"), "queued")).collect();
    let s = render("Big", "now", &many, &[]);
    assert!(s.len() <= LIMIT, "{} > {LIMIT}", s.len());
    assert!(s.contains("Left: Task number 0"), "{s}");
    assert!(s.contains("and 392 more"), "{s}");
}

async fn db() -> sqlx::SqlitePool {
    let p = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE projects (id TEXT, user_id TEXT, name TEXT, state_summary TEXT NOT NULL DEFAULT '', state_summary_at TEXT NOT NULL DEFAULT '')").execute(&p).await.unwrap();
    sqlx::query("CREATE TABLE tasks (id TEXT, user_id TEXT, project_id TEXT, parent_id TEXT, kind TEXT, title TEXT, state TEXT, position REAL DEFAULT 0)").execute(&p).await.unwrap();
    sqlx::query("CREATE TABLE task_events (id INTEGER PRIMARY KEY, task_id TEXT, user_id TEXT, at TEXT, kind TEXT, detail TEXT)").execute(&p).await.unwrap();
    sqlx::query("INSERT INTO projects (id, user_id, name) VALUES ('p', 'u', 'Shop')").execute(&p).await.unwrap();
    for (id, title, state) in [("a", "Login", "in_review"), ("b", "Pay", "queued")] {
        sqlx::query("INSERT INTO tasks (id, user_id, project_id, parent_id, kind, title, state) VALUES (?, 'u', 'p', NULL, 'task', ?, ?)").bind(id).bind(title).bind(state).execute(&p).await.unwrap();
    }
    p
}

#[tokio::test]
async fn refresh_rewrites_after_a_merge_and_reads_back() {
    let p = db().await;
    assert_eq!(refresh(&p, "p", "u").await.unwrap().contains("Tasks: 0 of 2 done"), true);
    sqlx::query("UPDATE tasks SET state = 'done' WHERE id = 'a'").execute(&p).await.unwrap();
    sqlx::query("INSERT INTO task_events (task_id, user_id, at, kind, detail) VALUES ('a', 'u', '2026-10-12', 'forge', '{\"pr\":\"https://f/pulls/3\",\"state\":{\"to\":\"done\"}}')").execute(&p).await.unwrap();
    let s = refresh(&p, "p", "u").await.unwrap();
    assert!(s.contains("Tasks: 1 of 2 done") && s.contains("Last merged: Login (https://f/pulls/3)"), "{s}");
    assert_eq!(summary_of(&p, "p", "u").await, s);
}

#[tokio::test]
async fn reading_an_unwritten_summary_builds_it_and_other_users_get_nothing() {
    let p = db().await;
    assert!(summary_of(&p, "p", "u").await.contains("Tasks: 0 of 2 done"));
    assert_eq!(summary_of(&p, "p", "someone-else").await, "");
}
