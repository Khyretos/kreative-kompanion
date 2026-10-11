//! KG-02 tests: plan parsing, sizing rules, dependency checks, and writing a plan into the task tree.
use super::*;
use serde_json::json;
use sqlx::SqlitePool;

fn task(key: &str, files: &[&str], lines: u32, deps: &[&str]) -> PlanTask {
    PlanTask {
        key: key.into(),
        title: format!("Task {key}"),
        goal: "Make it so.".into(),
        steps: vec!["Change the thing".into()],
        done_when: "The check passes".into(),
        files: files.iter().map(|f| f.to_string()).collect(),
        lines,
        depends_on: deps.iter().map(|d| d.to_string()).collect(),
    }
}

fn plan(tasks: Vec<Vec<PlanTask>>) -> Plan {
    Plan {
        title: "Goal".into(),
        goal: "Reach the goal.".into(),
        milestones: tasks
            .into_iter()
            .enumerate()
            .map(|(i, t)| Milestone { title: format!("M{i}"), goal: "Part.".into(), tasks: t })
            .collect(),
    }
}

#[test]
fn a_valid_plan_has_no_issues() {
    let p = plan(vec![vec![task("a", &["x.rs"], 50, &[]), task("b", &["y.rs"], 80, &["a"])], vec![task("c", &["z.rs"], 20, &["b"])]]);
    assert!(issues(&p, &Limits::default()).is_empty());
    assert!(oversize(&p, &Limits::default()).is_empty());
}

#[test]
fn sizing_rules_flag_too_many_files_and_lines() {
    let lim = Limits { max_files: 3, max_lines: 100 };
    let p = plan(vec![vec![
        task("ok", &["a", "b", "c"], 100, &[]),
        task("files", &["a", "b", "c", "d"], 10, &[]),
        task("lines", &["a"], 101, &[]),
    ]]);
    assert_eq!(oversize(&p, &lim), vec!["files".to_string(), "lines".to_string()]);
    let found = issues(&p, &lim);
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|i| i.kind == "oversize"));
}

#[test]
fn file_paths_count_once_each() {
    let lim = Limits { max_files: 2, max_lines: 100 };
    let p = plan(vec![vec![task("dup", &["a.rs", "a.rs", "b.rs"], 10, &[])]]);
    assert!(oversize(&p, &lim).is_empty());
}

#[test]
fn broken_structure_is_reported() {
    let mut no_done = task("a", &["x"], 10, &[]);
    no_done.done_when = "  ".into();
    let p = plan(vec![
        vec![no_done, task("a", &["x"], 10, &[]), task("b", &["x"], 10, &["ghost", "b"])],
        vec![],
    ]);
    let kinds: Vec<String> = issues(&p, &Limits::default()).into_iter().map(|i| i.kind).collect();
    for k in ["no_done_when", "duplicate_key", "unknown_dep", "self_dep", "empty_milestone"] {
        assert!(kinds.contains(&k.to_string()), "missing {k} in {kinds:?}");
    }
}

#[test]
fn dependency_loops_are_reported() {
    let p = plan(vec![vec![task("a", &["x"], 10, &["c"]), task("b", &["x"], 10, &["a"]), task("c", &["x"], 10, &["b"])]]);
    assert!(issues(&p, &Limits::default()).iter().any(|i| i.kind == "dep_loop"));
}

#[test]
fn parse_reads_json_from_a_fenced_answer() {
    let text = format!(
        "Here you go:\n```json\n{}\n```",
        json!({
            "title": "Game",
            "goal": "Ship a game.",
            "milestones": [{
                "title": "Core",
                "goal": "Movement",
                "tasks": [{"key": "t1", "title": "Move", "goal": "Player moves", "steps": ["Add input"], "done_when": "Arrow keys move",
                           "files": ["src/main.rs"], "lines": 60, "depends_on": []}]
            }]
        })
    );
    let p = parse(&text).unwrap();
    assert_eq!(p.title, "Game");
    assert_eq!(p.milestones[0].tasks[0].key, "t1");
    assert_eq!(p.milestones[0].tasks[0].lines, 60);
}

#[test]
fn parse_rejects_text_that_is_no_plan() {
    assert!(parse("I think you should start small.").is_err());
    assert!(parse("{\"title\": \"x\"}").is_err());
}

#[test]
fn schema_names_every_field_the_parser_reads() {
    let s = schema().to_string();
    for f in ["milestones", "tasks", "key", "done_when", "files", "lines", "depends_on", "steps"] {
        assert!(s.contains(f), "{f}");
    }
}

#[test]
fn a_task_description_passes_the_task_rules() {
    let d = description(&task("a", &["x.rs"], 10, &[]));
    assert!(d.contains("**Goal:**") && d.contains("1. Change the thing") && d.contains("**Done when:**"));
    assert!(crate::tasks::clean_description(&d).is_ok());
}

#[test]
fn the_fix_request_names_the_tasks_to_split() {
    let lim = Limits { max_files: 3, max_lines: 100 };
    let p = plan(vec![vec![task("big", &["a", "b", "c", "d"], 500, &[])]]);
    let msg = fix_request(&issues(&p, &lim), &lim);
    assert!(msg.contains("big") && msg.contains("3 files") && msg.contains("100 lines"));
}

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    for (u, p) in [("u1", "p1"), ("u2", "p2")] {
        sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES (?, ?, 'x', '2026')")
            .bind(u).bind(u).execute(&db).await.unwrap();
        sqlx::query("INSERT INTO projects (id, name, updated_at, user_id) VALUES (?, ?, '2026', ?)")
            .bind(p).bind(p).bind(u).execute(&db).await.unwrap();
    }
    db
}

#[tokio::test]
async fn applying_a_plan_builds_goal_milestones_tasks_and_deps() {
    let db = db().await;
    let p = plan(vec![vec![task("a", &["x"], 10, &[]), task("b", &["y"], 10, &["a"])], vec![task("c", &["z"], 10, &["b"])]]);
    let ids = apply(&db, "u1", "p1", &p, None).await.unwrap();
    assert_eq!(ids.len(), 1 + 2 + 3);
    let goal: (String, Option<String>) = sqlx::query_as("SELECT kind, parent_id FROM tasks WHERE id = ?").bind(&ids[0]).fetch_one(&db).await.unwrap();
    assert_eq!(goal, ("goal".into(), None));
    let miles: Vec<(String, String)> = sqlx::query_as("SELECT kind, parent_id FROM tasks WHERE kind = 'milestone' ORDER BY title").fetch_all(&db).await.unwrap();
    assert_eq!(miles.len(), 2);
    assert!(miles.iter().all(|(_, parent)| *parent == ids[0]));
    let b_deps: Vec<String> = sqlx::query_scalar("SELECT depends_on FROM task_deps WHERE task_id = (SELECT id FROM tasks WHERE title = 'Task b')")
        .fetch_all(&db).await.unwrap();
    let a_id: String = sqlx::query_scalar("SELECT id FROM tasks WHERE title = 'Task a'").fetch_one(&db).await.unwrap();
    assert_eq!(b_deps, vec![a_id]);
    let states: Vec<String> = sqlx::query_scalar("SELECT DISTINCT state FROM tasks").fetch_all(&db).await.unwrap();
    assert_eq!(states, vec!["queued".to_string()]);
}

#[tokio::test]
async fn applying_refuses_an_invalid_plan_and_writes_nothing() {
    let db = db().await;
    let p = plan(vec![vec![task("a", &["x"], 10, &["ghost"])]]);
    assert!(apply(&db, "u1", "p1", &p, None).await.is_err());
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks").fetch_one(&db).await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn applying_to_another_users_project_fails() {
    let db = db().await;
    let p = plan(vec![vec![task("a", &["x"], 10, &[])]]);
    assert!(apply(&db, "u1", "p2", &p, None).await.is_err());
}

#[tokio::test]
async fn a_replan_puts_new_tasks_under_the_failed_task_with_its_deps() {
    let db = db().await;
    let base = plan(vec![vec![task("a", &["x"], 10, &[]), task("b", &["y"], 10, &["a"])]]);
    apply(&db, "u1", "p1", &base, None).await.unwrap();
    let a: String = sqlx::query_scalar("SELECT id FROM tasks WHERE title = 'Task a'").fetch_one(&db).await.unwrap();
    let b: String = sqlx::query_scalar("SELECT id FROM tasks WHERE title = 'Task b'").fetch_one(&db).await.unwrap();
    let fix = plan(vec![vec![task("b1", &["y"], 10, &[]), task("b2", &["y"], 10, &["b1"])]]);
    let ids = apply_replan(&db, "u1", &b, &fix).await.unwrap();
    assert_eq!(ids.len(), 2);
    let kids: Vec<(String, String)> = sqlx::query_as("SELECT title, parent_id FROM tasks WHERE parent_id = ? ORDER BY title").bind(&b).fetch_all(&db).await.unwrap();
    assert_eq!(kids.iter().map(|k| k.0.as_str()).collect::<Vec<_>>(), ["Task b1", "Task b2"]);
    // The first new task inherits b's dependency on a; the second waits for the first.
    let b1_deps: Vec<String> = sqlx::query_scalar("SELECT depends_on FROM task_deps WHERE task_id = ?").bind(&ids[0]).fetch_all(&db).await.unwrap();
    assert_eq!(b1_deps, vec![a]);
    let b2_deps: Vec<String> = sqlx::query_scalar("SELECT depends_on FROM task_deps WHERE task_id = ?").bind(&ids[1]).fetch_all(&db).await.unwrap();
    assert_eq!(b2_deps, vec![ids[0].clone()]);
}

#[tokio::test]
async fn proposals_are_stored_per_user_and_can_be_closed() {
    let db = db().await;
    let p = plan(vec![vec![task("a", &["x"], 10, &[])]]);
    let id = store_proposal(&db, "u1", "p1", None, "plan", "", &p).await.unwrap();
    assert_eq!(pending(&db, "u1", "p1").await.unwrap().len(), 1);
    assert!(pending(&db, "u2", "p1").await.unwrap().is_empty());
    assert!(close_proposal(&db, "u2", &id, "dismissed").await.is_err());
    close_proposal(&db, "u1", &id, "dismissed").await.unwrap();
    assert!(pending(&db, "u1", "p1").await.unwrap().is_empty());
}

fn answer(p: &Plan) -> String {
    serde_json::to_string(p).unwrap()
}

/// A scripted model: returns the given answers in order and records the messages it was sent.
async fn run_generate(answers: Vec<String>) -> (Result<(Plan, Vec<Issue>), String>, Vec<Vec<Value>>) {
    let sent = std::sync::Mutex::new(Vec::new());
    let queue = std::sync::Mutex::new(answers.into_iter());
    let r = generate(
        |msgs: Vec<Value>| {
            sent.lock().unwrap().push(msgs);
            let next = queue.lock().unwrap().next().ok_or("no more answers".to_string());
            async move { next }
        },
        "plan it",
        "Build a game",
        &Limits { max_files: 3, max_lines: 100 },
    )
    .await;
    (r, sent.into_inner().unwrap())
}

#[tokio::test]
async fn generate_asks_again_until_every_task_is_sized() {
    let big = plan(vec![vec![task("big", &["a", "b", "c", "d"], 400, &[])]]);
    let split = plan(vec![vec![task("big1", &["a", "b"], 90, &[]), task("big2", &["c", "d"], 90, &["big1"])]]);
    let (r, sent) = run_generate(vec![answer(&big), answer(&split)]).await;
    let (plan, left) = r.unwrap();
    assert!(left.is_empty());
    assert_eq!(plan.milestones[0].tasks.len(), 2);
    assert_eq!(sent.len(), 2);
    let second = serde_json::to_string(&sent[1]).unwrap();
    assert!(second.contains("big") && second.contains("too big"));
}

#[tokio::test]
async fn generate_gives_up_after_three_rounds_and_returns_what_is_left() {
    let big = plan(vec![vec![task("big", &["a", "b", "c", "d"], 400, &[])]]);
    let (r, sent) = run_generate(vec![answer(&big), answer(&big), answer(&big), answer(&big)]).await;
    let (_, left) = r.unwrap();
    assert_eq!(sent.len(), 3);
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].kind, "oversize");
}

#[tokio::test]
async fn generate_recovers_from_an_answer_that_is_no_plan() {
    let ok = plan(vec![vec![task("a", &["x"], 10, &[])]]);
    let (r, sent) = run_generate(vec!["Sure! Let me think.".into(), answer(&ok)]).await;
    assert!(r.unwrap().1.is_empty());
    assert_eq!(sent.len(), 2);
}

#[tokio::test]
async fn generate_fails_when_the_model_never_makes_a_plan_or_errors() {
    let (r, _) = run_generate(vec!["no".into(), "still no".into(), "nope".into()]).await;
    assert!(r.is_err());
    let (r, _) = run_generate(vec![]).await;
    assert_eq!(r.unwrap_err(), "no more answers");
}
