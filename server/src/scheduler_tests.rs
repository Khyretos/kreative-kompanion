//! KG-03 tests: which queued task is ready, which machine gets it, budgets and the check verdict.
use super::*;
use std::collections::HashMap;
use crate::AppState;

fn states(list: &[(&str, &str)]) -> HashMap<String, String> {
    list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

fn cand(id: &str, position: f64, deps: &[&str]) -> Cand {
    Cand { id: id.into(), position, deps: deps.iter().map(|d| d.to_string()).collect(), machine: None, open_children: false }
}

fn mach(id: &str, free: bool) -> Mach {
    Mach { id: id.into(), free }
}

#[test]
fn a_task_is_ready_only_when_every_dependency_is_done() {
    let st = states(&[("a", "done"), ("b", "running"), ("c", "failed")]);
    assert!(ready(&cand("t", 1.0, &[]), &st));
    assert!(ready(&cand("t", 1.0, &["a"]), &st));
    assert!(!ready(&cand("t", 1.0, &["a", "b"]), &st));
    assert!(!ready(&cand("t", 1.0, &["c"]), &st), "a failed dependency never lets the task start");
    assert!(!ready(&cand("t", 1.0, &["gone"]), &st), "an unknown dependency counts as not done");
}

#[test]
fn a_task_with_unfinished_subtasks_does_not_run_itself() {
    let mut c = cand("t", 1.0, &[]);
    c.open_children = true;
    assert!(!ready(&c, &HashMap::new()));
}

#[test]
fn waiting_on_names_the_dependencies_that_are_not_done() {
    let st = states(&[("a", "done"), ("b", "running")]);
    assert_eq!(waiting_on(&cand("t", 1.0, &["a", "b", "z"]), &st), vec!["b".to_string(), "z".to_string()]);
}

#[test]
fn ready_tasks_go_in_order_to_free_machines_one_each() {
    let st = states(&[("d", "done")]);
    let cands = vec![cand("late", 3.0, &[]), cand("first", 1.0, &["d"]), cand("blocked", 2.0, &["nope"]), cand("second", 2.5, &[])];
    let machines = vec![mach("pc1", true), mach("pc2", true), mach("pc3", false)];
    let got = assign(&cands, &st, &machines);
    assert_eq!(got, vec![("first".to_string(), "pc1".to_string()), ("second".to_string(), "pc2".to_string())]);
}

#[test]
fn a_task_with_its_own_machine_waits_for_that_machine() {
    let mut c = cand("t", 1.0, &[]);
    c.machine = Some("pc2".into());
    assert!(assign(&[c.clone()], &HashMap::new(), &[mach("pc1", true), mach("pc2", false)]).is_empty());
    assert_eq!(assign(&[c], &HashMap::new(), &[mach("pc1", true), mach("pc2", true)]), vec![("t".to_string(), "pc2".to_string())]);
}

#[test]
fn a_machine_is_free_when_online_idle_and_its_gpu_has_no_job() {
    assert!(machine_free(Some(10), 0, 0));
    assert!(!machine_free(None, 0, 0), "never seen");
    assert!(!machine_free(Some(600), 0, 0), "offline");
    assert!(!machine_free(Some(10), 1, 0), "already running a task");
    assert!(!machine_free(Some(10), 0, 1), "its GPU is busy");
}

#[test]
fn budgets_fall_back_from_the_task_to_the_project_to_the_defaults() {
    let task = Budget { minutes: Some(5), rounds: None, tokens: None };
    let project = Budget { minutes: Some(30), rounds: Some(2), tokens: None };
    let b = task.or(project).or(Budget::DEFAULT);
    assert_eq!((b.minutes, b.rounds, b.tokens), (Some(5), Some(2), Budget::DEFAULT.tokens));
}

#[test]
fn a_budget_is_exceeded_by_time_or_tokens() {
    let b = Budget { minutes: Some(10), rounds: Some(3), tokens: Some(1000) };
    assert_eq!(exceeded(&b, 9, 999), None);
    assert!(exceeded(&b, 10, 0).unwrap().contains("time"));
    assert!(exceeded(&b, 1, 1000).unwrap().contains("token"));
    assert_eq!(exceeded(&Budget::default(), 9999, i64::MAX), None, "no limit set");
}

#[test]
fn fix_rounds_are_capped_by_the_budget_but_never_below_one() {
    assert_eq!(fix_rounds(3, Some(2)), 2);
    assert_eq!(fix_rounds(3, Some(9)), 3);
    assert_eq!(fix_rounds(3, Some(0)), 1);
    assert_eq!(fix_rounds(3, None), 3);
}

#[test]
fn only_exit_zero_passes_the_check() {
    assert_eq!(check_verdict("done", "ok\nexit: 0"), Verdict::Pass);
    assert!(matches!(check_verdict("done", "boom\nexit: 1"), Verdict::Fail(m) if m.contains("exit: 1") && m.contains("boom")));
    assert!(matches!(check_verdict("failed", "no such command"), Verdict::Fail(_)));
    assert!(matches!(check_verdict("done", "output without a code"), Verdict::NotRun(_)));
    assert!(matches!(check_verdict("refused", "not granted: shell"), Verdict::NotRun(_)));
    assert!(matches!(check_verdict("done", "exit: 0\nmore output\nexit: 2"), Verdict::Fail(_)), "the last exit line counts");
}

#[test]
fn the_check_failure_text_is_the_end_of_the_output() {
    let long = format!("{}\nlast line\nexit: 3", "x".repeat(20_000));
    let Verdict::Fail(m) = check_verdict("done", &long) else { panic!("should fail") };
    assert!(m.len() < 4_500);
    assert!(m.ends_with("exit: 3"));
}

#[test]
fn config_reads_partial_json_and_rejects_nothing_it_can_default() {
    let c = Config::parse("");
    assert!(!c.enabled);
    let c = Config::parse(r#"{"enabled":true,"machines":{"m1":"/work"},"check":"make test","budget":{"minutes":20}}"#);
    assert!(c.enabled);
    assert_eq!(c.machines.get("m1").map(String::as_str), Some("/work"));
    assert_eq!(c.check, "make test");
    assert_eq!(c.budget.minutes, Some(20));
    assert!(!Config::parse("not json").enabled);
}

#[test]
fn config_validation_wants_absolute_folders_and_positive_budgets() {
    let mut c = Config::default();
    c.machines.insert("m".into(), "relative".into());
    assert!(c.validate().is_err());
    c.machines.insert("m".into(), "/a/../b".into());
    assert!(c.validate().is_err());
    c.machines.insert("m".into(), "/work".into());
    assert!(c.validate().is_ok());
    c.budget.minutes = Some(0);
    assert!(c.validate().is_err());
}

async fn state() -> AppState {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u', 'u', 'x', '2026')").execute(&db).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, updated_at, user_id) VALUES ('p', 'p', '2026', 'u')").execute(&db).await.unwrap();
    sqlx::query("INSERT INTO machines (id, user_id, name, token_hash, created_at, last_seen) VALUES ('m1', 'u', 'pc1', 'h', '2026', ?)")
        .bind(crate::util::now())
        .execute(&db)
        .await
        .unwrap();
    AppState::for_tests(toml::from_str("").unwrap(), db)
}

async fn add_task(s: &AppState, id: &str, state: &str, check: Option<&str>) {
    sqlx::query("INSERT INTO tasks (id, project_id, title, description, state, updated_at, user_id, check_cmd) VALUES (?, 'p', ?, 'do it', ?, '2026', 'u', ?)")
        .bind(id).bind(id).bind(state).bind(check).execute(&s.db).await.unwrap();
}

async fn enable(s: &AppState, check: &str) {
    let c = Config { enabled: true, machines: [("m1".to_string(), "/work".to_string())].into(), check: check.into(), budget: Budget::default() };
    sqlx::query("UPDATE projects SET scheduler = ? WHERE id = 'p'").bind(serde_json::to_string(&c).unwrap()).execute(&s.db).await.unwrap();
}

async fn state_of(s: &AppState, id: &str) -> (String, String) {
    sqlx::query_as("SELECT state, step FROM tasks WHERE id = ?").bind(id).fetch_one(&s.db).await.unwrap()
}

#[tokio::test]
async fn a_disabled_scheduler_starts_nothing() {
    let s = state().await;
    add_task(&s, "t", "queued", Some("make test")).await;
    assert!(tick_project(&s, "u", "p").await.is_empty());
    assert_eq!(state_of(&s, "t").await.0, "queued");
}

#[tokio::test]
async fn a_task_without_a_check_is_held_not_started() {
    let s = state().await;
    enable(&s, "").await;
    add_task(&s, "t", "queued", None).await;
    assert!(tick_project(&s, "u", "p").await.is_empty());
    let (st, step) = state_of(&s, "t").await;
    assert_eq!(st, "needs_input");
    assert!(step.contains("no check command"));
}

#[tokio::test]
async fn a_task_waiting_on_an_unfinished_dependency_stays_queued() {
    let s = state().await;
    enable(&s, "make test").await;
    add_task(&s, "a", "needs_input", Some("x")).await;
    add_task(&s, "b", "queued", None).await;
    sqlx::query("INSERT INTO task_deps (task_id, depends_on) VALUES ('b', 'a')").execute(&s.db).await.unwrap();
    assert!(tick_project(&s, "u", "p").await.is_empty());
    assert_eq!(state_of(&s, "b").await.0, "queued");
}

#[tokio::test]
async fn a_busy_computer_takes_no_second_task() {
    let s = state().await;
    enable(&s, "make test").await;
    add_task(&s, "run", "running", Some("x")).await;
    sqlx::query("UPDATE tasks SET machine_id = 'm1' WHERE id = 'run'").execute(&s.db).await.unwrap();
    add_task(&s, "t", "queued", None).await;
    assert!(tick_project(&s, "u", "p").await.is_empty());
    assert_eq!(state_of(&s, "t").await.0, "queued");
}

#[tokio::test]
async fn a_ready_task_with_no_orchestrator_is_held_with_the_reason() {
    // The start itself fails (no orchestrator role is set): the task must not loop in the queue.
    let s = state().await;
    enable(&s, "make test").await;
    add_task(&s, "t", "queued", None).await;
    assert!(tick_project(&s, "u", "p").await.is_empty());
    let (st, step) = state_of(&s, "t").await;
    assert_eq!(st, "needs_input");
    assert!(step.contains("could not start"), "{step}");
}

#[tokio::test]
async fn a_task_budget_beats_the_project_budget() {
    let s = state().await;
    let c = Config { budget: Budget { minutes: Some(30), rounds: None, tokens: None }, ..Config::default() };
    sqlx::query("UPDATE projects SET scheduler = ? WHERE id = 'p'").bind(serde_json::to_string(&c).unwrap()).execute(&s.db).await.unwrap();
    add_task(&s, "t", "queued", None).await;
    assert_eq!(budget_of(&s, "t").await.minutes, Some(30));
    sqlx::query("UPDATE tasks SET budget_minutes = 5 WHERE id = 't'").execute(&s.db).await.unwrap();
    let b = budget_of(&s, "t").await;
    assert_eq!((b.minutes, b.rounds), (Some(5), Budget::DEFAULT.rounds));
}
