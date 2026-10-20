use super::{comfy, workflow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{path::{Path, PathBuf}, time::Duration};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Verdict {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub issues: Vec<String>,
}

pub async fn check(
    http: &reqwest::Client,
    base: &str,
    file: &Path,
    face_views: &[String],
    layout: Option<&str>,
) -> anyhow::Result<Verdict> {
    let bytes = tokio::fs::read(file).await?;
    let mut url = format!("{}/check", base.trim_end_matches('/'));
    let mut query = Vec::new();
    if !face_views.is_empty() {
        query.push(format!("face_views={}", face_views.join(",")));
    }
    if let Some(layout) = layout {
        query.push(format!("layout={layout}"));
    }
    if !query.is_empty() {
        url.push('?');
        url.push_str(&query.join("&"));
    }
    let r = http
        .post(&url)
        .header("Content-Type", "image/png")
        .body(bytes)
        .timeout(Duration::from_secs(180))
        .send()
        .await?;
    if !r.status().is_success() {
        let status = r.status();
        let text = r.text().await?;
        anyhow::bail!("anatomy check: {} {}", status, text);
    }
    Ok(r.json::<Verdict>().await?)
}

pub fn reseed(graph: &mut Value, p: &workflow::Param, seed: i64, add: i64, also: &[String]) {
    graph[p.node.as_str()]["inputs"][p.input.as_str()] = json!(seed);
    for (i, a) in also.iter().enumerate() {
        graph[a.as_str()]["inputs"][p.also[i].as_str()] = json!(seed + p.also[i].add);
    }
}

pub fn best(tries: &[Verdict]) -> usize {
    tries.iter().enumerate().min_by_key(|(_, v)| v.score).map(|(i, _)| i).unwrap_or(0)
}

pub fn summary(v: &Verdict, retries: u32) -> Value {
    json!({"ok": v.ok, "retries": retries, "issues": v.issues})
}

pub fn best(tries: &[Verdict]) -> usize {
    tries.iter().enumerate().min_by_key(|(_, v)| v.score).map(|(i, _)| i).unwrap_or(0)
}

pub fn summary(v: &Verdict, retries: u32) -> Value {
    json!({"ok": v.ok, "retries": retries, "issues": v.issues})
}

/// STU-C4: run the graph; with [check] and an anatomy_url, check the first picture and re-roll the seed up to
/// `retries` times, keeping the best try. Writes the kept "seed" and "check" into `used`.
#[allow(clippy::too_many_arguments)]
pub async fn run_checked(http: &reqwest::Client, anatomy_url: Option<&str>, w: &workflow::Workflow, url: &str, graph: &mut Value, out_dir: &Path, id: &str, used: &mut Map<String, Value>) -> anyhow::Result<Vec<PathBuf>> {
    let limit = Duration::from_secs(1800);
    let first = comfy::run(http, url, graph, &w.outputs, out_dir, id, limit).await?;

    let (Some(c), Some(base)) = (&w.check, anatomy_url) else { return Ok(first) };
    let Some(seed_p) = w.params.iter().find(|p| p.kind == "seed") else { return Ok(first) };
    // Helmet pairs keep one seed for both sides.
    if used.get("pair").and_then(Value::as_str).is_some_and(|p| !p.is_empty()) {
        return Ok(first);
    }

    if !first.is_empty() {
        let mut seed = used.get("seed").and_then(Value::as_i64).unwrap_or(0);
        let v = match check(http, base, &first[0], &c.face_views).await {
            Ok(v) => v,
            Err(e) => { tracing::warn!("{e:#}"); used.insert("check".into(), json!({"error": e.to_string()})); return Ok(first); }
        };
        let mut tries: Vec<(Vec<PathBuf>, Verdict, i64)> = vec![(first, v, seed)];

        let mut n = 0u32;
        while !tries[tries.len() - 1].1.ok && n < c.retries {
            n += 1;
            seed = rand::random::<u32>() as i64;
            reseed(graph, seed_p, seed);
            let files = match comfy::run(http, url, graph, &w.outputs, out_dir, &format!("{id}-retry{n}"), limit).await {
                Ok(files) => files,
                Err(e) => { tracing::warn!("retry {n}: {e:#}"); break; }
            };
            let Some(f) = files.first().cloned() else { break };
            match check(http, base, &f, &c.face_views).await {
                Ok(v) => tries.push((files, v, seed)),
                Err(e) => { tracing::warn!("{e:#}"); for f in &files { let _ = tokio::fs::remove_file(f).await; } break; }
            }
        }

        let verdicts: Vec<Verdict> = tries.iter().map(|t| t.1.clone()).collect();
        let b = best(&verdicts);
        let retries = (tries.len() - 1) as u32;
        for (i, t) in tries.iter().enumerate() { if i != b { for f in &t.0 { let _ = tokio::fs::remove_file(f).await; } } }
        let (files, v, kept) = tries.swap_remove(b);
        reseed(graph, seed_p, kept);
        used.insert("seed".into(), json!(kept));
        used.insert("check".into(), summary(&v, retries));
        return Ok(files);
    }

    Ok(first)
}

#[cfg(test)]
#[path = "anatomy_tests.rs"]
mod tests;
