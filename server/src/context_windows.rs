//! CTX-02: the context size of each model, in tokens, from the best source there is. In this order:
//! set by an admin (manual) > learned from a server's refusal > the provider's own `context_window` > detected from the
//! server > a known size for common cloud models. None of them: Kompanion treats the model as a small one (8192 tokens,
//! see attach_read). The window feeds the context meter, when the chat is summarised, and how much of an attached file is
//! shown to the model and read in parts.

use std::{collections::HashMap, time::Duration};
use axum::{Extension, Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use crate::{AppState, auth::User, config::ProviderConfig, context::known_window, context_detect, error::{ApiError, ApiResult}};

/// How long a detection stays fresh before the next answer checks again in the background (a server is reconfigured now and then).
const STALE_SECS: i64 = 6 * 3600;
/// The first answer with a model nobody has asked about waits this long for the server to say.
const FIRST_WAIT: Duration = Duration::from_secs(8);

/// What is known about one model.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Entry {
    pub manual: Option<i64>,
    pub detected: Option<i64>,
    pub detected_from: Option<String>,
    pub detected_at: Option<i64>,
    pub learned: Option<i64>,
    pub learned_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Manual,
    Learned,
    Provider,
    Detected,
    Known,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Manual => "manual",
            Source::Learned => "learned",
            Source::Provider => "provider",
            Source::Detected => "detected",
            Source::Known => "known",
        }
    }
}

pub type Windows = std::sync::Arc<std::sync::RwLock<HashMap<(String, String), Entry>>>;

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// The size to use and where it came from.
pub fn resolve(entry: Option<&Entry>, p: Option<&ProviderConfig>, model: &str) -> Option<(i64, Source)> {
    let e = entry.cloned().unwrap_or_default();
    e.manual.map(|n| (n, Source::Manual))
        .or_else(|| e.learned.map(|n| (n, Source::Learned)))
        .or_else(|| p.and_then(|p| p.context_window).map(|n| (i64::from(n), Source::Provider)))
        .or_else(|| e.detected.map(|n| (n, Source::Detected)))
        .or_else(|| known_window(model).map(|n| (n, Source::Known)))
}

fn entry_of(s: &AppState, provider_id: &str, model: &str) -> Option<Entry> {
    s.windows.read().ok()?.get(&(provider_id.to_string(), model.to_string())).cloned()
}

/// The context window of a model in tokens (None: not known).
pub fn window_for(s: &AppState, provider_id: &str, model: &str) -> Option<i64> {
    resolve(entry_of(s, provider_id, model).as_ref(), s.provider(provider_id).as_ref(), model).map(|r| r.0)
}

type Row = (String, String, Option<i64>, Option<i64>, Option<String>, Option<i64>, Option<i64>, Option<i64>);

/// Reads what was saved into the cache (at start).
pub async fn load(s: &AppState) {
    let rows: Vec<Row> = sqlx::query_as("SELECT provider_id, model_id, manual, detected, detected_from, detected_at, learned, learned_at FROM model_context")
        .fetch_all(&s.db).await.unwrap_or_default();
    if let Ok(mut map) = s.windows.write() {
        *map = rows.into_iter().map(|r| ((r.0, r.1), Entry { manual: r.2, detected: r.3, detected_from: r.4, detected_at: r.5, learned: r.6, learned_at: r.7 })).collect();
    }
}

async fn save(s: &AppState, provider_id: &str, model: &str, e: &Entry) -> ApiResult<()> {
    sqlx::query("INSERT INTO model_context (provider_id, model_id, manual, detected, detected_from, detected_at, learned, learned_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(provider_id, model_id) DO UPDATE SET manual = ?3, detected = ?4, detected_from = ?5, detected_at = ?6, learned = ?7, learned_at = ?8")
        .bind(provider_id).bind(model).bind(e.manual).bind(e.detected).bind(&e.detected_from).bind(e.detected_at).bind(e.learned).bind(e.learned_at)
        .execute(&s.db).await?;
    if let Ok(mut map) = s.windows.write() {
        map.insert((provider_id.to_string(), model.to_string()), e.clone());
    }
    Ok(())
}

/// Asks the server and keeps what it says. An answer that finds nothing keeps an earlier value; the time is kept either way,
/// so a server with nothing to say is not asked again at every message. `forget_learned` is for an explicit Detect.
async fn probe(s: &AppState, provider_id: &str, model: &str, forget_learned: bool) -> Entry {
    let mut e = entry_of(s, provider_id, model).unwrap_or_default();
    if let Some(p) = s.provider(provider_id) {
        if let Some(f) = context_detect::detect(&s.http, &p, model).await {
            e.detected = Some(f.tokens);
            e.detected_from = Some(f.from);
        }
        e.detected_at = Some(now());
    }
    if forget_learned {
        e.learned = None;
        e.learned_at = None;
    }
    save(s, provider_id, model, &e).await.ok();
    e
}

/// Called before an answer: a model nobody has asked about is asked now (a few seconds at most, once); one asked long ago is
/// checked again in the background. A cloud model whose size is known by name is not asked at all.
pub async fn ensure(s: &AppState, provider_id: &str, model: &str) {
    let Some(p) = s.provider(provider_id) else { return };
    match entry_of(s, provider_id, model) {
        None if !p.local && known_window(model).is_some() => {}
        None => { tokio::time::timeout(FIRST_WAIT, probe(s, provider_id, model, false)).await.ok(); }
        Some(e) if e.detected_at.is_none_or(|t| now() - t > STALE_SECS) && (p.local || known_window(model).is_none()) => {
            let (s, provider_id, model) = (s.clone(), provider_id.to_string(), model.to_string());
            tokio::spawn(async move { probe(&s, &provider_id, &model, false).await; });
        }
        Some(_) => {}
    }
}

/// An explicit Detect in Settings: asks now and forgets what a refusal taught.
pub async fn detect_now(s: &AppState, provider_id: &str, model: &str) -> Entry {
    probe(s, provider_id, model, true).await
}

/// A model nobody has asked about: asked when Settings lists it (so the list shows the size), without waiting long.
pub async fn probe_if_new(s: &AppState, p: &ProviderConfig, model: &str) {
    if entry_of(s, &p.id, model).is_some() || (!p.local && known_window(model).is_some()) { return }
    tokio::time::timeout(FIRST_WAIT, probe(s, &p.id, model, false)).await.ok();
}

/// A refused prompt that names a limit: keeps it (only ever lower) and returns the sentence to show after the error.
/// `chars` is the length of the prompt that was sent.
pub async fn learn(s: &AppState, provider_id: &str, model: &str, error: &str, chars: usize) -> Option<String> {
    let n = context_detect::limit_in(error, chars)?;
    let mut e = entry_of(s, provider_id, model).unwrap_or_default();
    let lower = e.learned.is_none_or(|old| n < old);
    if lower {
        e.learned = Some(n);
        e.learned_at = Some(now());
        save(s, provider_id, model, &e).await.ok();
    }
    Some(match e.manual {
        Some(m) if m > n => format!("The server accepts about {n} tokens for {model}, but the size set for it in Settings (Connected models) is {m}: lower it so long prompts fit."),
        _ => format!("Kompanion saved about {n} tokens as the context size of {model} and keeps prompts within it from now on (Settings, Connected models). Send your message again."),
    })
}

/// The fields of one model in the providers list (the effective size, where it came from, and each source).
pub fn model_fields(s: &AppState, p: &ProviderConfig, model: &str) -> Value {
    let e = entry_of(s, &p.id, model).unwrap_or_default();
    let r = resolve(Some(&e), Some(p), model);
    json!({
        "id": model,
        "contextTokens": r.map(|x| x.0),
        "contextSource": r.map(|x| x.1.as_str()),
        "contextManual": e.manual,
        "contextDetected": e.detected,
        "contextFrom": e.detected_from,
        "contextLearned": e.learned,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeForm {
    provider_id: String,
    model_id: String,
    tokens: Option<i64>,
}

fn check(s: &AppState, f: &SizeForm) -> ApiResult<ProviderConfig> {
    let p = s.provider(&f.provider_id).ok_or(ApiError::NotFound)?;
    if f.model_id.is_empty() || f.model_id.chars().count() > 200 { return Err(ApiError::BadRequest("Pick a model.".into())) }
    Ok(p)
}

/// PUT /api/admin/context-size: sets the size of a model by hand, or clears it (tokens null) to go back to automatic.
pub async fn put_size(State(s): State<AppState>, Extension(u): Extension<User>, Json(f): Json<SizeForm>) -> ApiResult<Json<Value>> {
    crate::admin::require_admin(&s, &u).await?;
    let p = check(&s, &f)?;
    if f.tokens.is_some_and(|n| !(context_detect::MIN..=context_detect::MAX).contains(&n)) {
        return Err(ApiError::BadRequest(format!("The context size is a number of tokens from {} to {}, or empty for automatic.", context_detect::MIN, context_detect::MAX)));
    }
    let mut e = entry_of(&s, &f.provider_id, &f.model_id).unwrap_or_default();
    e.manual = f.tokens;
    save(&s, &f.provider_id, &f.model_id, &e).await?;
    Ok(Json(model_fields(&s, &p, &f.model_id)))
}

/// POST /api/admin/context-size/detect: asks the server now, and forgets what a refusal taught.
pub async fn detect(State(s): State<AppState>, Extension(u): Extension<User>, Json(f): Json<SizeForm>) -> ApiResult<Json<Value>> {
    crate::admin::require_admin(&s, &u).await?;
    let p = check(&s, &f)?;
    let e = detect_now(&s, &f.provider_id, &f.model_id).await;
    let mut out = model_fields(&s, &p, &f.model_id);
    out["found"] = json!(e.detected.is_some());
    Ok(Json(out))
}

#[cfg(test)]
#[path = "context_windows_tests.rs"]
mod tests;
