//! KG-08: capability matrix. For each model and task type: how many tasks of that type it tried on its own
//! and how many it finished. Finished task runs and benchmark results feed it; the task router reads it to
//! skip a worker that is known to fail a type when a configured `[[ladder]]` rung is known to pass it.
use std::collections::HashMap;

use axum::{Extension, Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::{AppState, auth::User, config::LadderRung, error::{ApiError, ApiResult}};

/// Tries needed before the matrix says anything about a model.
pub const MIN_TRIES: i64 = 3;
pub const TYPES: [&str; 5] = ["bugfix", "tests", "docs", "refactor", "feature"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stat {
    pub tried: i64,
    pub passed: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Unknown,
    Passes,
    Fails,
}

/// (provider, model, task type) to its counts.
pub type Matrix = HashMap<(String, String, String), Stat>;

/// pure: the task type from the title and description (first match wins, "feature" otherwise).
pub fn task_type(title: &str, description: &str) -> &'static str {
    let t = format!("{title} {description}").to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| t.contains(w));
    if has(&["fix", "bug", "crash", "regression", "failing"]) {
        "bugfix"
    } else if has(&["test"]) {
        "tests"
    } else if has(&["readme", "docs", "document", "comment"]) {
        "docs"
    } else if has(&["refactor", "rename", "clean up", "cleanup", "extract"]) {
        "refactor"
    } else {
        "feature"
    }
}

/// pure: passes alone at 80 % or more, fails at 40 % or less, both only after `MIN_TRIES`.
pub fn verdict(s: Stat) -> Verdict {
    if s.tried < MIN_TRIES {
        Verdict::Unknown
    } else if s.passed * 5 >= s.tried * 4 {
        Verdict::Passes
    } else if s.passed * 5 <= s.tried * 2 {
        Verdict::Fails
    } else {
        Verdict::Unknown
    }
}

fn verdict_of(m: &Matrix, provider: &str, model: &str, ttype: &str) -> Verdict {
    m.get(&(provider.to_string(), model.to_string(), ttype.to_string())).copied().map(verdict).unwrap_or(Verdict::Unknown)
}

/// pure: when the worker role is known to fail this type, the first configured rung known to pass it.
/// None keeps the worker (unknown, fine, or nothing better is known).
pub fn pick_start<'a>(worker: &(String, String), rungs: &'a [LadderRung], ttype: &str, m: &Matrix, configured: impl Fn(&str) -> bool) -> Option<&'a LadderRung> {
    if verdict_of(m, &worker.0, &worker.1, ttype) != Verdict::Fails {
        return None;
    }
    rungs.iter().find(|r| configured(&r.provider) && verdict_of(m, &r.provider, &r.model, ttype) == Verdict::Passes)
}

/// pure: one line per model, its types with counts and the verdict.
pub fn text(m: &Matrix) -> String {
    let mut models: Vec<(&String, &String)> = m.keys().map(|(p, md, _)| (p, md)).collect();
    models.sort();
    models.dedup();
    let mut out = String::new();
    for (p, md) in models {
        let parts: Vec<String> = TYPES
            .iter()
            .filter_map(|t| {
                let s = m.get(&(p.clone(), md.clone(), t.to_string()))?;
                let v = match verdict(*s) {
                    Verdict::Passes => "passes",
                    Verdict::Fails => "fails",
                    Verdict::Unknown if s.tried < MIN_TRIES => "too few runs",
                    Verdict::Unknown => "mixed",
                };
                Some(format!("{t} {}/{} ({v})", s.passed, s.tried))
            })
            .collect();
        out.push_str(&format!("{p}/{md}: {}\n", parts.join(", ")));
    }
    out.trim_end().to_string()
}

pub async fn load(db: &SqlitePool) -> Matrix {
    let rows: Vec<(String, String, String, i64, i64)> = sqlx::query_as("SELECT provider, model, task_type, tried, passed FROM capability").fetch_all(db).await.unwrap_or_default();
    rows.into_iter().map(|(p, m, t, tried, passed)| ((p, m, t), Stat { tried, passed })).collect()
}

pub async fn record(db: &SqlitePool, provider: &str, model: &str, ttype: &str, passed: bool) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO capability (provider, model, task_type, tried, passed, updated_at) VALUES (?, ?, ?, 1, ?, ?)
         ON CONFLICT (provider, model, task_type) DO UPDATE SET tried = tried + 1, passed = passed + excluded.passed, updated_at = excluded.updated_at",
    )
    .bind(provider)
    .bind(model)
    .bind(ttype)
    .bind(passed as i64)
    .bind(crate::util::now())
    .execute(db)
    .await?;
    Ok(())
}

/// A benchmark run: one (type, passed) pair per fixture.
pub async fn record_bench(db: &SqlitePool, provider: &str, model: &str, results: &[(String, bool)]) -> sqlx::Result<()> {
    for (t, ok) in results {
        record(db, provider, model, t, *ok).await?;
    }
    Ok(())
}

/// GET /api/capabilities
pub async fn get(State(s): State<AppState>, Extension(_u): Extension<User>) -> ApiResult<Json<Value>> {
    let m = load(&s.db).await;
    let rows: Vec<Value> = m
        .iter()
        .map(|((p, md, t), st)| json!({ "provider": p, "model": md, "taskType": t, "tried": st.tried, "passed": st.passed, "verdict": format!("{:?}", verdict(*st)).to_lowercase() }))
        .collect();
    Ok(Json(json!({ "matrix": rows, "text": text(&m) })))
}

#[derive(Deserialize)]
pub struct BenchBody {
    provider: String,
    model: String,
    results: Vec<BenchResult>,
}

#[derive(Deserialize)]
pub struct BenchResult {
    #[serde(rename = "type")]
    task_type: String,
    passed: bool,
}

/// POST /api/capabilities/bench: the result of a benchmark run for one model.
pub async fn bench(State(s): State<AppState>, Extension(_u): Extension<User>, Json(b): Json<BenchBody>) -> ApiResult<Json<Value>> {
    if b.results.is_empty() || b.results.len() > 100 || b.results.iter().any(|r| !TYPES.contains(&r.task_type.as_str())) {
        return Err(ApiError::BadRequest(format!("Send 1-100 results, each with a type of {}.", TYPES.join(", "))));
    }
    let rows: Vec<(String, bool)> = b.results.into_iter().map(|r| (r.task_type, r.passed)).collect();
    record_bench(&s.db, &b.provider, &b.model, &rows).await?;
    Ok(Json(json!({ "recorded": rows.len() })))
}

#[cfg(test)]
#[path = "capmatrix_tests.rs"]
mod tests;
