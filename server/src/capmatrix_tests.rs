//! KG-08 tests: task types from words, verdicts need enough tries, the router skips a worker that is known to
//! fail a type only for a rung that is known to pass it, and the table counts runs and benchmark results.
use super::*;

fn st(tried: i64, passed: i64) -> Stat {
    Stat { tried, passed }
}

#[test]
fn task_types_come_from_the_words() {
    assert_eq!(task_type("Fix crash on login", ""), "bugfix");
    assert_eq!(task_type("Add tests for parser", "cover the edge cases"), "tests");
    assert_eq!(task_type("Update README", "document the flags"), "docs");
    assert_eq!(task_type("Rename helper", "refactor the module"), "refactor");
    assert_eq!(task_type("Build the export page", "new screen"), "feature");
}

#[test]
fn a_verdict_needs_three_tries() {
    assert_eq!(verdict(st(2, 2)), Verdict::Unknown);
    assert_eq!(verdict(st(5, 4)), Verdict::Passes);
    assert_eq!(verdict(st(5, 2)), Verdict::Fails);
    assert_eq!(verdict(st(5, 3)), Verdict::Unknown);
    assert_eq!(verdict(st(0, 0)), Verdict::Unknown);
}

fn rung(p: &str, m: &str) -> crate::config::LadderRung {
    crate::config::LadderRung { name: None, provider: p.into(), model: m.into(), fix_rounds: None }
}

fn matrix(rows: &[(&str, &str, &str, i64, i64)]) -> Matrix {
    rows.iter().map(|(p, m, t, tried, passed)| ((p.to_string(), m.to_string(), t.to_string()), st(*tried, *passed))).collect()
}

#[test]
fn the_router_skips_a_failing_worker_for_a_passing_rung_only() {
    let rungs = vec![rung("ovms", "Coder"), rung("anthropic", "sonnet")];
    let worker = ("ovms".to_string(), "Qwen9B".to_string());
    let all = |_: &str| true;
    let m = matrix(&[("ovms", "Qwen9B", "bugfix", 6, 1), ("anthropic", "sonnet", "bugfix", 4, 4), ("ovms", "Coder", "bugfix", 4, 1)]);
    assert_eq!(pick_start(&worker, &rungs, "bugfix", &m, all).unwrap().model, "sonnet");
    // the worker is fine for tests, or unknown: no change
    assert!(pick_start(&worker, &rungs, "tests", &m, all).is_none());
    // worker fails but no rung is known to pass: keep the worker
    let weak = matrix(&[("ovms", "Qwen9B", "bugfix", 6, 1)]);
    assert!(pick_start(&worker, &rungs, "bugfix", &weak, all).is_none());
    // a rung whose provider is not configured is never picked
    assert!(pick_start(&worker, &rungs, "bugfix", &m, |p| p != "anthropic").is_none());
}

#[test]
fn the_text_lists_each_model_by_type() {
    let m = matrix(&[("ovms", "Coder", "bugfix", 5, 4), ("ovms", "Coder", "docs", 1, 1)]);
    let t = text(&m);
    assert!(t.contains("ovms/Coder: bugfix 4/5 (passes), docs 1/1 (too few runs)"), "{t}");
}

async fn db() -> sqlx::SqlitePool {
    let p = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::query(include_str!("../migrations/0129_capability.sql")).execute(&p).await.unwrap();
    p
}

#[tokio::test]
async fn runs_and_benchmarks_are_counted() {
    let p = db().await;
    for ok in [true, true, false] {
        record(&p, "ovms", "Coder", "bugfix", ok).await.unwrap();
    }
    record_bench(&p, "ovms", "Coder", &[("bugfix".into(), true), ("docs".into(), false)]).await.unwrap();
    let m = load(&p).await;
    assert_eq!(m[&("ovms".to_string(), "Coder".to_string(), "bugfix".to_string())], st(4, 3));
    assert_eq!(m[&("ovms".to_string(), "Coder".to_string(), "docs".to_string())], st(1, 0));
}
