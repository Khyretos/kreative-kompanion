//! CTX-02: the context size of a model, found out instead of guessed. Two sources, both without running a model:
//! asking the server (`detect`: its model list, llama.cpp /props, Ollama /api/ps and /api/show, LM Studio, the Anthropic
//! API) and reading the limit a server names when it refuses a prompt as too long (`limit_in`). A server that says nothing
//! (OVMS, for one) gets neither; its size is then set by hand (context_windows).

use std::{sync::LazyLock, time::Duration};
use regex::Regex;
use serde_json::{Value, json};
use crate::{config::{ProviderConfig, ProviderKind}, llm::{authorize, join}};

/// Sizes outside this are not believed (a field that holds something else, or a typo).
pub const MIN: i64 = 1_000;
pub const MAX: i64 = 10_000_000;
/// One probe's time: a server that does not answer quickly has no such endpoint, or is down.
const PROBE: Duration = Duration::from_secs(4);
/// Ollama's context when a model does not set `num_ctx` (a model loaded for longer gets what /api/ps says).
const OLLAMA_DEFAULT: i64 = 4_096;
/// Of the size worked out from a refusal, this share is kept (the estimate is rough; the margin keeps the next prompt under).
const SAFETY: f64 = 0.85;
/// Characters per token the refusal arithmetic assumes (the budgets of attach_read use the same).
const CHARS_PER_TOKEN: f64 = 3.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub tokens: i64,
    pub from: String,
}

fn sane(n: i64) -> Option<i64> {
    (MIN..=MAX).contains(&n).then_some(n)
}

fn num(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).or_else(|| v.as_str()?.trim().parse().ok()).and_then(sane)
}

/// The server's own address without the API version: `http://host:11434/v1` is `http://host:11434`.
pub fn root(base: &str) -> String {
    static VERSION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/v\d+(?:beta\d*)?/?$").expect("regex"));
    VERSION.replace(base.trim_end_matches('/'), "").trim_end_matches('/').to_string()
}

// ---------------------------------------------------------------- what each server's answer looks like

/// An OpenAI-style `GET /models` entry for the model: vLLM says `max_model_len`, OpenRouter and others `context_length`.
pub fn from_models_list(body: &Value, model: &str) -> Option<i64> {
    let item = body["data"].as_array()?.iter().find(|m| m["id"].as_str() == Some(model))?;
    ["max_model_len", "context_length", "context_window", "max_context_length", "max_input_tokens"].iter().find_map(|k| num(&item[*k]))
}

/// llama.cpp's `GET /props`: the context one slot got when the server started.
pub fn from_props(body: &Value) -> Option<i64> {
    num(&body["default_generation_settings"]["n_ctx"]).or_else(|| num(&body["n_ctx"]))
}

fn same_model(name: &str, model: &str) -> bool {
    let strip = |s: &str| s.strip_suffix(":latest").unwrap_or(s).to_string();
    strip(name) == strip(model)
}

/// Ollama's `GET /api/ps`: a model that is loaded now, with the context it was loaded for.
pub fn from_ollama_ps(body: &Value, model: &str) -> Option<i64> {
    let item = body["models"].as_array()?.iter().find(|m| m["name"].as_str().is_some_and(|n| same_model(n, model)) || m["model"].as_str().is_some_and(|n| same_model(n, model)))?;
    num(&item["context_length"])
}

/// Ollama's `POST /api/show`: `num_ctx` when the model sets it; else Ollama's default, which is what it will use (the model's
/// own maximum is not: a model that supports 131072 still runs with 4096 unless asked). None when the answer is not Ollama's.
pub fn from_ollama_show(body: &Value) -> Option<(i64, &'static str)> {
    static NUM_CTX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*num_ctx\s+(\d+)").expect("regex"));
    body.get("model_info")?;
    let set = body["parameters"].as_str().and_then(|p| NUM_CTX.captures(p)).and_then(|c| c[1].parse().ok()).and_then(sane);
    Some(match set {
        Some(n) => (n, "Ollama (the model's num_ctx)"),
        None => (OLLAMA_DEFAULT, "Ollama (its default, as the model sets no num_ctx)"),
    })
}

/// LM Studio's `GET /api/v0/models`: only a model that is loaded has a context in use.
pub fn from_lmstudio(body: &Value, model: &str) -> Option<i64> {
    let item = body["data"].as_array()?.iter().find(|m| m["id"].as_str() == Some(model) && m["state"].as_str() == Some("loaded"))?;
    num(&item["loaded_context_length"])
}

/// Anthropic's `GET /v1/models/{id}`.
pub fn from_anthropic(body: &Value) -> Option<i64> {
    num(&body["max_input_tokens"])
}

// ---------------------------------------------------------------- asking

async fn get_json(http: &reqwest::Client, p: &ProviderConfig, url: String) -> Option<Value> {
    let resp = tokio::time::timeout(PROBE, authorize(http.get(url), p).send()).await.ok()?.ok()?;
    if !resp.status().is_success() { return None }
    tokio::time::timeout(PROBE, resp.json::<Value>()).await.ok()?.ok()
}

fn found(tokens: i64, from: &str) -> Option<Found> {
    Some(Found { tokens, from: from.to_string() })
}

/// Asks the provider's server what context the model has. Only reads: no chat request, nothing that loads a model.
pub async fn detect(http: &reqwest::Client, p: &ProviderConfig, model: &str) -> Option<Found> {
    if model.is_empty() || model.chars().any(|c| c.is_whitespace() || matches!(c, '?' | '#' | '\\')) { return None }
    if p.kind == ProviderKind::Anthropic {
        let body = get_json(http, p, join(&p.base_url, &format!("v1/models/{model}"))).await?;
        return found(from_anthropic(&body)?, "the Anthropic API");
    }
    let root = root(&p.base_url);
    if let Some(n) = get_json(http, p, join(&p.base_url, "models")).await.and_then(|b| from_models_list(&b, model)) { return found(n, "the server's model list") }
    if let Some(n) = get_json(http, p, join(&root, "props")).await.and_then(|b| from_props(&b)) { return found(n, "llama.cpp (/props)") }
    if let Some(n) = get_json(http, p, join(&root, "api/ps")).await.and_then(|b| from_ollama_ps(&b, model)) { return found(n, "Ollama (loaded now)") }
    if let Some(n) = get_json(http, p, join(&root, "api/v0/models")).await.and_then(|b| from_lmstudio(&b, model)) { return found(n, "LM Studio (loaded now)") }
    let show = tokio::time::timeout(PROBE, authorize(http.post(join(&root, "api/show")), p).json(&json!({ "model": model })).send()).await.ok()?.ok()?;
    if !show.status().is_success() { return None }
    let body: Value = tokio::time::timeout(PROBE, show.json()).await.ok()?.ok()?;
    let (n, from) = from_ollama_show(&body)?;
    found(n, from)
}

// ---------------------------------------------------------------- what a refusal says

/// The context size a server names when it refuses a prompt as too long, from the error text (`chars` is the length of the
/// prompt that was refused). A server that counts its own way (ovms-queue's guard says "about N tokens, the limit is M") gets
/// the size scaled by how the prompt measured against it, with a margin.
pub fn limit_in(text: &str, chars: usize) -> Option<i64> {
    static GUARD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"about (\d+) tokens, the limit is (\d+)").expect("regex"));
    static PLAIN: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        [
            r"maximum context length is (\d+)",                 // OpenAI, vLLM, DeepSeek, Mistral
            r"context length of (\d+) tokens",                  // some OpenAI-compatible servers
            r"(\d+) maximum context length",                    // Mistral
            r"> (\d+) maximum",                                 // Anthropic: "prompt is too long: 213000 tokens > 200000 maximum"
            r"maximum number of tokens allowed \((\d+)\)",      // Gemini
            r#""n_ctx"\s*:\s*(\d+)"#,                           // llama.cpp: exceed_context_size_error
            r"context (?:window|size) (?:of|is) (\d+)",         // LM Studio and others
        ].iter().map(|p| Regex::new(p).expect("regex")).collect()
    });
    let lower = text.to_lowercase();
    if let Some(c) = GUARD.captures(&lower) {
        let (about, limit): (f64, f64) = (c[1].parse().ok()?, c[2].parse().ok()?);
        if chars == 0 || about <= 0.0 || limit <= 0.0 || about <= limit { return None }
        return sane((limit * (chars as f64 / CHARS_PER_TOKEN) / about * SAFETY) as i64);
    }
    PLAIN.iter().find_map(|r| r.captures(&lower).and_then(|c| c[1].parse().ok())).and_then(sane)
}

#[cfg(test)]
#[path = "context_detect_tests.rs"]
mod tests;
