//! BUG-02: every Studio run has a time limit and a progress check, and Studio off stops it at once.
use std::{collections::HashMap, future::Future, sync::{Arc, LazyLock, Mutex}, time::{Duration, Instant}};
use anyhow::{Result, bail};
use serde_json::Value;
use tokio::sync::Notify;

pub fn in_queue(q: &Value, id: &str) -> bool {
    let queue_running = q.get("queue_running").and_then(|v| v.as_array());
    let queue_pending = q.get("queue_pending").and_then(|v| v.as_array());
    let mut queues = queue_running.into_iter().chain(queue_pending.into_iter()).flatten();
    queues.any(|entry| {
        entry.as_array().map_or(false, |arr| arr.get(1).map_or(false, |v| v.as_str() == Some(id)))
    })
}

pub fn audio_alive(health: &Value) -> bool {
    health.get("busy").and_then(|v| v.as_bool()).unwrap_or(false)
}

pub const STALL: Duration = Duration::from_secs(90);

pub fn limit(kind: &str) -> Duration {
    match kind {
        "sfx" => Duration::from_secs(300),
        "music" => Duration::from_secs(900),
        _ => Duration::from_secs(2100),
    }
}

static RUNS: LazyLock<Mutex<HashMap<String, (String, Arc<Notify>)>>> = LazyLock::new(Default::default);

pub struct Watch { id: String, stop: Arc<Notify> }

pub fn watch(id: &str, gpu: &str) -> Watch {
    let stop = Arc::new(Notify::new());
    RUNS.lock().unwrap().insert(id.to_string(), (gpu.to_string(), stop.clone()));
    Watch { id: id.to_string(), stop }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let mut map = RUNS.lock().unwrap();
        map.remove(&self.id);
    }
}

pub fn stop_gpus(gpus: &[String]) -> usize {
    let mut count = 0;
    for (gpu, notify) in RUNS.lock().unwrap().values() {
        if gpus.contains(gpu) {
            notify.notify_one();
            count += 1;
        }
    }
    count
}

pub async fn guard<T>(w: &Watch, limit: Duration, fut: impl Future<Output = Result<T>>) -> Result<T> {
    tokio::select! {
        r = fut => r,
        _ = tokio::time::sleep(limit) => bail!("stopped: no result within the time limit of {} min", limit.as_secs().div_ceil(60)),
        _ = w.stop.notified() => bail!("stopped: the Studio was turned off"),
    }
}

pub struct Progress { since: Instant, stall: Duration }

impl Progress {
    pub fn new(stall: Duration, now: Instant) -> Self {
        Progress { since: now, stall }
    }

    pub fn tick(&mut self, alive: bool, now: Instant) -> Result<()> {
        if alive {
            self.since = now;
            return Ok(());
        }
        if now.duration_since(self.since) > self.stall {
            bail!("stopped: no progress for {} s", self.stall.as_secs());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "watchdog_tests.rs"]
mod tests;
