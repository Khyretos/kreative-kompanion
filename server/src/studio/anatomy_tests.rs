use super::*;
use axum::{Router, body::Bytes, extract::{Query, State}, http::StatusCode, routing::post, Json};
use serde_json::json;
use std::{collections::HashMap, future::IntoFuture, sync::{Arc, Mutex}};

#[derive(Default)]
struct Got {
    body: Mutex<Vec<u8>>,
    query: Mutex<HashMap<String, String>>,
}

async fn fake(code: u16, answer: serde_json::Value) -> (String, Arc<Got>) {
    let got = Arc::new(Got::default());
    let app = Router::new()
        .route(
            "/check",
            post(move |State(g): State<Arc<Got>>, Query(q): Query<HashMap<String, String>>, body: Bytes| {
                let answer = answer.clone();
                async move {
                    *g.body.lock().unwrap() = body.to_vec();
                    *g.query.lock().unwrap() = q;
                    (StatusCode::from_u16(code).unwrap(), Json(answer))
                }
            }),
        )
        .with_state(got.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(axum::serve(listener, app).into_future());
    (url, got)
}

fn png() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("anatomy-{}.png", std::process::id()));
    std::fs::write(&p, b"\x89PNG fake").unwrap();
    p
}

#[tokio::test]
async fn check_posts_the_picture_and_reads_the_verdict() {
    let (url, got) = fake(200, json!({"ok": false, "score": 2, "issues": ["back: face seen (head facing the wrong way)", "left: 3 hands"], "cells": []})).await;
    let views = vec!["front".to_string(), "left".to_string(), "right".to_string()];
    let v = check(&reqwest::Client::new(), &url, &png(), &views).await.unwrap();
    assert!(!v.ok);
    assert_eq!(v.score, 2);
    assert_eq!(v.issues, vec!["back: face seen (head facing the wrong way)".to_string(), "left: 3 hands".to_string()]);
    assert_eq!(got.body.lock().unwrap().as_slice(), b"\x89PNG fake");
    assert_eq!(got.query.lock().unwrap().get("face_views").map(String::as_str), Some("front,left,right"));
}

#[tokio::test]
async fn no_face_views_sends_no_query() {
    let (url, got) = fake(200, json!({"ok": true, "score": 0, "issues": []})).await;
    let v = check(&reqwest::Client::new(), &url, &png(), &[]).await.unwrap();
    assert!(v.ok);
    assert!(got.query.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_checker_error_is_an_error() {
    let (url, _) = fake(500, json!({"error": "model missing"})).await;
    let e = check(&reqwest::Client::new(), &url, &png(), &[]).await.unwrap_err();
    assert!(e.to_string().contains("model missing"), "{e}");
}

#[test]
fn reseed_sets_the_seed_and_its_offsets() {
    let p: workflow::Param = toml::from_str(
        "name = \"seed\"\ntype = \"seed\"\ndefault = -1\nnode = \"7\"\ninput = \"seed\"\nalso = [{ node = \"9\", input = \"seed\", add = 1 }, { node = \"13\", input = \"noise_seed\", add = 2 }]",
    )
    .unwrap();
    let mut g = json!({"7": {"inputs": {"seed": 1}}, "9": {"inputs": {"seed": 2}}, "13": {"inputs": {"noise_seed": 3}}});
    reseed(&mut g, &p, 500);
    assert_eq!(g["7"]["inputs"]["seed"], json!(500));
    assert_eq!(g["9"]["inputs"]["seed"], json!(501));
    assert_eq!(g["13"]["inputs"]["noise_seed"], json!(502));
}

fn v(score: u32) -> Verdict {
    Verdict { ok: score == 0, score, issues: vec!["x".to_string(); score as usize] }
}

#[test]
fn best_is_the_lowest_score_and_the_first_on_a_tie() {
    assert_eq!(best(&[v(2), v(1), v(1)]), 1);
    assert_eq!(best(&[v(0)]), 0);
    assert_eq!(best(&[v(3), v(2), v(4), v(0)]), 3);
}

#[test]
fn summary_says_how_often_it_retried() {
    assert_eq!(summary(&v(0), 2), json!({"ok": true, "retries": 2, "issues": []}));
    assert_eq!(summary(&v(1), 3), json!({"ok": false, "retries": 3, "issues": ["x"]}));
}

// ---- run_checked: a fake ComfyUI and anatomy checker on one port ----

struct Fake {
    scores: Mutex<Vec<u32>>,
    seeds: Mutex<Vec<i64>>,
    fail_check: bool,
    /// ComfyUI refuses every prompt after this many.
    prompts: usize,
}

async fn fake_both(scores: Vec<u32>, fail_check: bool) -> (String, Arc<Fake>) {
    fake_both_limited(scores, fail_check, usize::MAX).await
}

async fn fake_both_limited(scores: Vec<u32>, fail_check: bool, prompts: usize) -> (String, Arc<Fake>) {
    use axum::routing::get;
    let f = Arc::new(Fake { scores: Mutex::new(scores), seeds: Mutex::new(vec![]), fail_check, prompts });
    let app = Router::new()
        .route("/system_stats", get(|| async { StatusCode::OK }))
        .route(
            "/prompt",
            post(|State(f): State<Arc<Fake>>, Json(b): Json<serde_json::Value>| async move {
                let mut seeds = f.seeds.lock().unwrap();
                if seeds.len() >= f.prompts {
                    return (StatusCode::BAD_REQUEST, Json(json!({"error": "out of memory"})));
                }
                seeds.push(b["prompt"]["7"]["inputs"]["seed"].as_i64().unwrap());
                (StatusCode::OK, Json(json!({"prompt_id": "p1"})))
            }),
        )
        .route(
            "/history/{id}",
            get(|| async {
                Json(json!({"p1": {"status": {"status_str": "success", "completed": true},
                "outputs": {"10": {"images": [{"filename": "a.png", "subfolder": "kompanion", "type": "output"}]}}}}))
            }),
        )
        .route("/view", get(|| async { b"PNG".to_vec() }))
        .route(
            "/check",
            post(|State(f): State<Arc<Fake>>| async move {
                if f.fail_check {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "no models"})));
                }
                let s = f.scores.lock().unwrap().remove(0);
                (StatusCode::OK, Json(json!({"ok": s == 0, "score": s, "issues": vec!["back: face seen (head facing the wrong way)"; s as usize]})))
            }),
        )
        .with_state(f.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(axum::serve(listener, app).into_future());
    (url, f)
}

fn checked_wf() -> workflow::Workflow {
    toml::from_str(
        "title = \"OC sheet\"\noutputs = [\"10\"]\n[check]\nretries = 3\nface_views = [\"front\"]\n[[param]]\nname = \"seed\"\ntype = \"seed\"\ndefault = -1\nnode = \"7\"\ninput = \"seed\"\nalso = [{ node = \"9\", input = \"seed\", add = 1 }]\n",
    )
    .unwrap()
}

fn out_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("anatomy-run-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn files_in(d: &std::path::Path) -> usize {
    std::fs::read_dir(d).map(|r| r.count()).unwrap_or(0)
}

fn used_with_seed() -> serde_json::Map<String, serde_json::Value> {
    json!({"seed": 11, "prompt": "a knight"}).as_object().unwrap().clone()
}

#[tokio::test]
async fn a_failing_sheet_is_rerolled_until_it_passes() {
    let (url, f) = fake_both(vec![2, 1, 0], false).await;
    let dir = out_dir("pass");
    let mut g = json!({"7": {"inputs": {"seed": 11}}, "9": {"inputs": {"seed": 12}}});
    let mut used = used_with_seed();
    let files = run_checked(&reqwest::Client::new(), Some(url.as_str()), &checked_wf(), &url, &mut g, &dir, "run1", &mut used).await.unwrap();
    let seeds = f.seeds.lock().unwrap().clone();
    assert_eq!(seeds.len(), 3);
    assert_eq!(seeds[0], 11);
    assert_eq!(used["seed"], json!(seeds[2]));
    assert_eq!(g["9"]["inputs"]["seed"], json!(seeds[2] + 1));
    assert_eq!(used["check"], json!({"ok": true, "retries": 2, "issues": []}));
    assert_eq!(files.len(), 1);
    assert!(files[0].exists());
    assert_eq!(files_in(&dir), 1, "the other tries' files are removed");
}

#[tokio::test]
async fn after_three_retries_the_best_try_is_kept() {
    let (url, f) = fake_both(vec![3, 1, 2, 2], false).await;
    let dir = out_dir("best");
    let mut g = json!({"7": {"inputs": {"seed": 11}}, "9": {"inputs": {"seed": 12}}});
    let mut used = used_with_seed();
    let files = run_checked(&reqwest::Client::new(), Some(url.as_str()), &checked_wf(), &url, &mut g, &dir, "run2", &mut used).await.unwrap();
    let seeds = f.seeds.lock().unwrap().clone();
    assert_eq!(seeds.len(), 4);
    assert_eq!(used["seed"], json!(seeds[1]));
    assert_eq!(used["check"], json!({"ok": false, "retries": 3, "issues": ["back: face seen (head facing the wrong way)"]}));
    assert_eq!(files.len(), 1);
    assert!(files[0].exists());
    assert_eq!(files_in(&dir), 1);
}

#[tokio::test]
async fn a_helmet_pair_and_a_missing_checker_are_not_checked() {
    let (url, f) = fake_both(vec![2, 2], false).await;
    let wf = checked_wf();
    let mut g = json!({"7": {"inputs": {"seed": 11}}, "9": {"inputs": {"seed": 12}}});
    let mut used = used_with_seed();
    used.insert("pair".into(), json!("p-1"));
    run_checked(&reqwest::Client::new(), Some(url.as_str()), &wf, &url, &mut g, &out_dir("pair"), "run3", &mut used).await.unwrap();
    let mut used2 = used_with_seed();
    run_checked(&reqwest::Client::new(), None, &wf, &url, &mut g, &out_dir("none"), "run4", &mut used2).await.unwrap();
    assert_eq!(f.seeds.lock().unwrap().len(), 2);
    assert!(used.get("check").is_none());
    assert!(used2.get("check").is_none());
    assert_eq!(used2["seed"], json!(11));
}

#[tokio::test]
async fn a_checker_error_keeps_the_first_picture() {
    let (url, f) = fake_both(vec![], true).await;
    let dir = out_dir("err");
    let mut g = json!({"7": {"inputs": {"seed": 11}}, "9": {"inputs": {"seed": 12}}});
    let mut used = used_with_seed();
    let files = run_checked(&reqwest::Client::new(), Some(url.as_str()), &checked_wf(), &url, &mut g, &dir, "run5", &mut used).await.unwrap();
    assert_eq!(f.seeds.lock().unwrap().len(), 1);
    assert_eq!(files.len(), 1);
    assert!(used["check"]["error"].as_str().unwrap().contains("no models"));
}

#[tokio::test]
async fn a_failed_retry_keeps_the_best_try_so_far() {
    let (url, f) = fake_both_limited(vec![2, 1], false, 2).await;
    let dir = out_dir("retryfail");
    let mut g = json!({"7": {"inputs": {"seed": 11}}, "9": {"inputs": {"seed": 12}}});
    let mut used = used_with_seed();
    let files = run_checked(&reqwest::Client::new(), Some(url.as_str()), &checked_wf(), &url, &mut g, &dir, "run6", &mut used).await.unwrap();
    let seeds = f.seeds.lock().unwrap().clone();
    assert_eq!(seeds.len(), 2);
    assert_eq!(used["seed"], json!(seeds[1]));
    assert_eq!(used["check"], json!({"ok": false, "retries": 1, "issues": ["back: face seen (head facing the wrong way)"]}));
    assert_eq!(files.len(), 1);
    assert!(files[0].exists());
    assert_eq!(files_in(&dir), 1);
}
