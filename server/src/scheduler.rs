//! KG-03: the task queue scheduler. Every few seconds it looks at each project that has the scheduler
//! switched on, takes the queued tasks whose dependencies are all done, and starts them one per free
//! computer (online, running no task, its GPU without a job). A task counts as done only when its
//! check command exits 0 (run by `taskrun`, see `run_check`); budgets (time, tokens, fix rounds) end
//! a run that goes on too long. The pure rules are here and tested; the loop only wires them to the DB.
use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{AppState, access, events::Event, taskrun, util};

/// A computer is "online" when its runner reported this recently.
const ONLINE_SECS: i64 = 90;

/// How often the loop looks at the queues.
const EVERY: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Budget {
    pub minutes: Option<i64>,
    pub rounds: Option<i64>,
    pub tokens: Option<i64>,
}

impl Budget {
    pub const DEFAULT: Budget = Budget { minutes: Some(60), rounds: Some(3), tokens: Some(400_000) };

    /// This budget's own limits, the other's where this one has none.
    pub fn or(self, other: Budget) -> Budget {
        Budget { minutes: self.minutes.or(other.minutes), rounds: self.rounds.or(other.rounds), tokens: self.tokens.or(other.tokens) }
    }
}

/// Why a run must stop now, or None while it is inside its budget.
pub fn exceeded(b: &Budget, elapsed_minutes: i64, tokens_used: i64) -> Option<String> {
    if let Some(m) = b.minutes.filter(|m| elapsed_minutes >= *m) {
        return Some(format!("time budget of {m} min used up"));
    }
    if let Some(t) = b.tokens.filter(|t| tokens_used >= *t) {
        return Some(format!("token budget of {t} used up"));
    }
    None
}

/// The fix rounds a run may use: its effort's, capped by the budget, never below one.
pub fn fix_rounds(by_effort: usize, cap: Option<i64>) -> usize {
    match cap {
        Some(c) => by_effort.min(c.max(1) as usize),
        None => by_effort,
    }
}

/// A project's scheduler settings (stored as JSON in projects.scheduler).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub enabled: bool,
    /// Computer id -> the project's folder on that computer.
    pub machines: HashMap<String, String>,
    /// The check command for tasks that have none of their own.
    pub check: String,
    pub budget: Budget,
}

impl Config {
    pub fn parse(text: &str) -> Config {
        serde_json::from_str(text).unwrap_or_default()
    }

    pub fn validate(&self) -> Result<(), String> {
        for (m, folder) in &self.machines {
            if !folder.starts_with('/') || folder.split('/').any(|c| c == "..") {
                return Err(format!("The folder for computer {m} must be an absolute path without \"..\"."));
            }
        }
        for v in [self.budget.minutes, self.budget.rounds, self.budget.tokens].into_iter().flatten() {
            if v < 1 {
                return Err("Budgets must be 1 or more.".into());
            }
        }
        Ok(())
    }
}

/// A queued task as the scheduler sees it.
#[derive(Debug, Clone)]
pub struct Cand {
    pub id: String,
    pub position: f64,
    pub deps: Vec<String>,
    /// The computer the task asks for; None: any free one.
    pub machine: Option<String>,
    /// Subtasks that are not done yet (the parent waits for them).
    pub open_children: bool,
}

#[derive(Debug, Clone)]
pub struct Mach {
    pub id: String,
    pub free: bool,
}

/// Dependencies that are not done yet (unknown ones too).
pub fn waiting_on(c: &Cand, states: &HashMap<String, String>) -> Vec<String> {
    c.deps.iter().filter(|d| states.get(*d).map(String::as_str) != Some("done")).cloned().collect()
}

pub fn ready(c: &Cand, states: &HashMap<String, String>) -> bool {
    !c.open_children && waiting_on(c, states).is_empty()
}

/// Ready tasks in queue order, each to the first free computer it may use; a computer gets one task.
pub fn assign(cands: &[Cand], states: &HashMap<String, String>, machines: &[Mach]) -> Vec<(String, String)> {
    let mut order: Vec<&Cand> = cands.iter().filter(|c| ready(c, states)).collect();
    order.sort_by(|a, b| a.position.total_cmp(&b.position));
    let mut taken: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for c in order {
        let pick = machines.iter().find(|m| m.free && !taken.contains(&m.id.as_str()) && c.machine.as_deref().is_none_or(|w| w == m.id));
        if let Some(m) = pick {
            taken.push(&m.id);
            out.push((c.id.clone(), m.id.clone()));
        }
    }
    out
}

/// A computer takes a task when its runner is online, it runs no task and its GPU has no job.
pub fn machine_free(seen_secs_ago: Option<i64>, running_tasks: i64, gpu_jobs: i64) -> bool {
    seen_secs_ago.is_some_and(|a| a <= ONLINE_SECS) && running_tasks == 0 && gpu_jobs == 0
}

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Pass,
    /// The check ran and failed: the end of its output, for the fix round.
    Fail(String),
    /// The check could not be run at all (not granted, no exit code): the task is not done either.
    NotRun(String),
}

/// What a shell job's end state and output say about the check: only "exit: 0" passes.
pub fn check_verdict(state: &str, output: &str) -> Verdict {
    let tail = {
        let mut start = output.len().saturating_sub(4000);
        while !output.is_char_boundary(start) {
            start += 1;
        }
        output[start..].trim().to_string()
    };
    match state {
        "done" => match output.lines().rev().find_map(|l| l.trim().strip_prefix("exit:")).map(|c| c.trim().to_string()) {
            Some(code) if code == "0" => Verdict::Pass,
            Some(_) => Verdict::Fail(tail),
            None => Verdict::NotRun("the check printed no exit code".into()),
        },
        "failed" => Verdict::Fail(tail),
        other => Verdict::NotRun(format!("the check was {other}: {}", tail.lines().next().unwrap_or(""))),
    }
}

/// Runs a check command on a computer and reads its exit code. Waits up to `wait`.
pub async fn run_check(s: &AppState, user_id: &str, machine_id: &str, folder: &str, cmd: &str, wait: Duration) -> Verdict {
    s.host.watch(user_id);
    let job = match access::queue_job(&s.db, machine_id, user_id, &json!({ "tool": "shell", "cwd": folder, "command": cmd }), None).await {
        Ok(j) => j,
        Err(e) => return Verdict::NotRun(format!("could not queue the check: {e}")),
    };
    let deadline = tokio::time::Instant::now() + wait;
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let row: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT state, result FROM machine_jobs WHERE id = ?").bind(&job).fetch_optional(&s.db).await.ok().flatten();
        if let Some((state, out)) = row
            && matches!(state.as_str(), "done" | "failed" | "refused")
        {
            return check_verdict(&state, &out.unwrap_or_default());
        }
    }
    let _ = sqlx::query("DELETE FROM machine_jobs WHERE id = ? AND state = 'queued'").bind(&job).execute(&s.db).await;
    Verdict::NotRun("the check did not finish in time".into())
}

/// The budget a task runs under: its own limits, the project's, then the defaults.
pub async fn budget_of(s: &AppState, task_id: &str) -> Budget {
    type BudgetRow = (Option<i64>, Option<i64>, Option<i64>, Option<String>);
    let row: Option<BudgetRow> = sqlx::query_as(
        "SELECT t.budget_minutes, t.budget_rounds, t.budget_tokens, p.scheduler FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?",
    )
    .bind(task_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((minutes, rounds, tokens, cfg)) = row else { return Budget::DEFAULT };
    Budget { minutes, rounds, tokens }.or(Config::parse(&cfg.unwrap_or_default()).budget).or(Budget::DEFAULT)
}

/// Tokens a run has used so far (all its model calls).
pub async fn tokens_used(s: &AppState, run_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COALESCE(SUM(COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0)), 0) FROM calls WHERE run_id = ?")
        .bind(run_id)
        .fetch_one(&s.db)
        .await
        .unwrap_or(0)
}

static TICK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// One round for one project: starts what is ready. Returns the (task, computer) pairs started.
pub async fn tick_project(s: &AppState, user_id: &str, project_id: &str) -> Vec<(String, String)> {
    let _one_at_a_time = TICK.lock().await;
    let cfg_text: Option<String> = sqlx::query_scalar("SELECT scheduler FROM projects WHERE id = ? AND user_id = ?")
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let mut cfg = Config::parse(&cfg_text.unwrap_or_default());
    if cfg.enabled {
        crate::devsettings_web::fill_scheduler(&s.db, project_id, user_id, &mut cfg).await;
    }
    if !cfg.enabled || cfg.machines.is_empty() {
        return vec![];
    }
    let rows: Vec<(String, f64, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, position, machine_id, check_cmd FROM tasks WHERE project_id = ? AND user_id = ? AND state = 'queued' AND kind = 'task'",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    if rows.is_empty() {
        return vec![];
    }
    let states: HashMap<String, String> = sqlx::query_as("SELECT id, state FROM tasks WHERE project_id = ?")
        .bind(project_id)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();
    let mut cands = Vec::new();
    let mut checks: HashMap<String, Option<String>> = HashMap::new();
    for (id, position, machine, check) in rows {
        let deps: Vec<(String,)> = sqlx::query_as("SELECT depends_on FROM task_deps WHERE task_id = ?").bind(&id).fetch_all(&s.db).await.unwrap_or_default();
        let children: i64 = sqlx::query_scalar("SELECT count(*) FROM tasks WHERE parent_id = ? AND state <> 'done'")
            .bind(&id)
            .fetch_one(&s.db)
            .await
            .unwrap_or(0);
        checks.insert(id.clone(), check.filter(|c| !c.trim().is_empty()));
        cands.push(Cand { id, position, deps: deps.into_iter().map(|d| d.0).collect(), machine: machine.filter(|m| cfg.machines.contains_key(m)), open_children: children > 0 });
    }
    let mut machines = Vec::new();
    for id in cfg.machines.keys() {
        machines.push(Mach { id: id.clone(), free: free_now(s, user_id, id).await });
    }
    let mut started = Vec::new();
    for (task, machine) in assign(&cands, &states, &machines) {
        let check = checks.get(&task).cloned().flatten().or_else(|| Some(cfg.check.clone()).filter(|c| !c.trim().is_empty()));
        let Some(check) = check else {
            hold(s, user_id, &task, "no check command: set one on the task or in the project's scheduler").await;
            continue;
        };
        let body = taskrun::StartBody {
            machine_id: machine.clone(),
            folder: cfg.machines[&machine].clone(),
            check,
            protected: vec![],
            tests_may_change: false,
            effort: None,
        };
        match taskrun::start_task(s, user_id, &task, body, true).await {
            Ok(()) => started.push((task, machine)),
            Err(e) => hold(s, user_id, &task, &format!("the scheduler could not start it: {e}")).await,
        }
    }
    started
}

/// A task the scheduler cannot start goes to needs_input with the reason, instead of being retried every round.
async fn hold(s: &AppState, user_id: &str, task_id: &str, why: &str) {
    let _ = sqlx::query("UPDATE tasks SET state = 'needs_input', step = ?, updated_at = ? WHERE id = ? AND state = 'queued'")
        .bind(why)
        .bind(util::now())
        .bind(task_id)
        .execute(&s.db)
        .await;
    s.bus.send(user_id, Event::Changed { what: "tasks", machine_id: None });
}

async fn free_now(s: &AppState, user_id: &str, machine_id: &str) -> bool {
    let row: Option<(String, Option<String>)> = sqlx::query_as("SELECT name, last_seen FROM machines WHERE id = ? AND user_id = ?")
        .bind(machine_id)
        .bind(user_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let Some((name, seen)) = row else { return false };
    let age = seen.and_then(|t| time::OffsetDateTime::parse(&t, &time::format_description::well_known::Rfc3339).ok()).map(|t| (time::OffsetDateTime::now_utc() - t).whole_seconds());
    let running: i64 = sqlx::query_scalar("SELECT count(*) FROM tasks WHERE machine_id = ? AND state = 'running'")
        .bind(machine_id)
        .fetch_one(&s.db)
        .await
        .unwrap_or(0);
    let gpus: Vec<String> = s.config.gpus.iter().filter(|g| g.machine == name).map(|g| g.id.clone()).collect();
    let mut gpu_jobs = 0;
    for g in gpus {
        gpu_jobs += sqlx::query_scalar::<_, i64>("SELECT count(*) FROM gpu_job WHERE gpu = ? AND state IN ('queued', 'running')")
            .bind(g)
            .fetch_one(&s.db)
            .await
            .unwrap_or(0);
    }
    machine_free(age, running, gpu_jobs)
}

/// The loop: every few seconds, each project with the scheduler on gets one round.
pub fn spawn(s: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(EVERY).await;
            let projects: Vec<(String, String)> =
                sqlx::query_as("SELECT id, user_id FROM projects WHERE scheduler LIKE '%\"enabled\":true%'").fetch_all(&s.db).await.unwrap_or_default();
            for (project, user) in projects {
                let started = tick_project(&s, &user, &project).await;
                for (task, machine) in started {
                    tracing::info!("scheduler: started task {task} on {machine}");
                }
            }
        }
    });
}

#[cfg(test)]
#[path = "scheduler_tests.rs"]
mod tests;
