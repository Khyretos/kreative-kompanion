//! PROV-UI: model providers an admin adds, changes and removes in Settings (custom_providers),
//! used next to the [[provider]] entries of kompanion.toml. A key is never stored or shown: only
//! the environment variable or the secret file that holds it, and both are limited to names and
//! folders meant for keys, so a provider cannot be pointed at another secret of the server.

use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{AppState, auth::User, config::{ProviderConfig, ProviderKind}, error::{ApiError, ApiResult}, util};

/// Folders a key file may be in: Docker secrets, and `secrets/` next to the database.
pub fn secret_dirs(s: &AppState) -> Vec<std::path::PathBuf> {
    let mut dirs = vec![std::path::PathBuf::from("/run/secrets")];
    if let Some(parent) = std::path::Path::new(&s.config.database).parent() {
        dirs.push(parent.join("secrets"));
    }
    dirs
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Form {
    name: String,
    kind: String,
    base_url: String,
    #[serde(default)]
    local: bool,
    #[serde(default)]
    api_key_env: Option<String>,
    #[serde(default)]
    api_key_file: Option<String>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    context_window: Option<u32>,
}

/// An environment variable meant for a key: upper case, ending in _KEY or _TOKEN.
fn key_env_ok(name: &str) -> bool {
    let shape = name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    shape && (name.ends_with("_KEY") || name.ends_with("_TOKEN"))
}

/// A key file inside one of the secret folders (absolute, no `..`).
fn key_file_ok(path: &str, dirs: &[std::path::PathBuf]) -> bool {
    let p = std::path::Path::new(path);
    p.is_absolute()
        && !p.components().any(|c| matches!(c, std::path::Component::ParentDir))
        && dirs.iter().any(|d| p.starts_with(d) && p != d.as_path())
}

/// The form as a provider, or why it cannot be one.
fn check(s: &AppState, id: &str, f: &Form) -> ApiResult<ProviderConfig> {
    let bad = |m: &str| Err(ApiError::BadRequest(m.into()));
    let name = f.name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() || name.chars().count() > 60 {
        return bad("Give the provider a name of up to 60 characters.");
    }
    let kind: ProviderKind = match serde_json::from_value(json!(f.kind)) {
        Ok(k) => k,
        Err(_) => return bad("The kind is openai-compatible or anthropic."),
    };
    let base_url = f.base_url.trim().trim_end_matches('/').to_string();
    let parsed = reqwest::Url::parse(&base_url);
    if base_url.len() > 300 || !matches!(parsed.as_ref().map(|u| u.scheme()), Ok("http" | "https")) || parsed.as_ref().is_ok_and(|u| u.host_str().is_none()) {
        return bad("The address starts with https:// (or http:// on your own network).");
    }
    let env = f.api_key_env.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let file = f.api_key_file.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    if env.is_some() && file.is_some() {
        return bad("Pick one place for the key: an environment variable or a secret file.");
    }
    if env.as_deref().is_some_and(|v| !key_env_ok(v)) {
        return bad("The key's environment variable is in capitals and ends in _KEY or _TOKEN, like XAI_API_KEY.");
    }
    let dirs = secret_dirs(s);
    if file.as_deref().is_some_and(|v| !key_file_ok(v, &dirs)) {
        let shown: Vec<String> = dirs.iter().map(|d| d.display().to_string()).collect();
        return Err(ApiError::BadRequest(format!("The key file must be in {}.", shown.join(" or "))));
    }
    if f.max_tokens.is_some_and(|n| !(1..=200_000).contains(&n)) {
        return bad("Output tokens per answer: 1 to 200,000, or empty for the default.");
    }
    if f.context_window.is_some_and(|n| !(1_000..=10_000_000).contains(&n)) {
        return bad("The context window: 1,000 to 10,000,000 tokens, or empty to look it up.");
    }
    Ok(ProviderConfig {
        id: id.to_string(),
        name,
        kind,
        base_url,
        local: f.local,
        api_key_env: env,
        api_key_file: file,
        extra_body: None,
        effort: Default::default(),
        ttl: None,
        max_tokens: f.max_tokens,
        context_window: f.context_window,
    })
}

/// A new id from the name ("xAI Grok" -> "xai-grok"), unique among all providers.
fn new_id(s: &AppState, name: &str) -> String {
    let mut base: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    while base.contains("--") {
        base = base.replace("--", "-");
    }
    let base = base.trim_matches('-').chars().take(40).collect::<String>();
    let base = if base.is_empty() { "provider".to_string() } else { base };
    let mut id = base.clone();
    let mut n = 2;
    while s.provider(&id).is_some() {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

type Row = (String, String, String, String, bool, Option<String>, Option<String>, Option<i64>, Option<i64>);

fn from_row(r: Row) -> Option<ProviderConfig> {
    Some(ProviderConfig {
        id: r.0,
        name: r.1,
        kind: serde_json::from_value(json!(r.2)).ok()?,
        base_url: r.3,
        local: r.4,
        api_key_env: r.5,
        api_key_file: r.6,
        extra_body: None,
        effort: Default::default(),
        ttl: None,
        max_tokens: r.7.and_then(|n| u32::try_from(n).ok()),
        context_window: r.8.and_then(|n| u32::try_from(n).ok()),
    })
}

/// Reads the added providers into the registry (at start and after every change).
pub async fn load(s: &AppState) {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, name, kind, base_url, local, api_key_env, api_key_file, max_tokens, context_window FROM custom_providers ORDER BY created_at, id",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<ProviderConfig> = rows.into_iter().filter_map(from_row).filter(|p| s.config.provider(&p.id).is_none()).collect();
    if let Ok(mut w) = s.custom_providers.write() {
        *w = list;
    }
}

fn shown(p: &ProviderConfig) -> Value {
    json!({
        "id": p.id, "name": p.name,
        "kind": match p.kind { ProviderKind::OpenaiCompatible => "openai-compatible", ProviderKind::Anthropic => "anthropic" },
        "baseUrl": p.base_url, "local": p.local, "apiKeyEnv": p.api_key_env, "apiKeyFile": p.api_key_file,
        "maxTokens": p.max_tokens, "contextWindow": p.context_window, "hasKey": p.api_key().is_some(),
    })
}

/// GET /api/admin/providers: the added providers (never a key), and where key files may live.
pub async fn list(State(s): State<AppState>, Extension(u): Extension<User>) -> ApiResult<Json<Value>> {
    crate::admin::require_admin(&s, &u).await?;
    let list: Vec<Value> = s.custom_providers.read().map(|l| l.iter().map(shown).collect()).unwrap_or_default();
    let dirs: Vec<String> = secret_dirs(&s).iter().map(|d| d.display().to_string()).collect();
    Ok(Json(json!({ "providers": list, "secretDirs": dirs })))
}

async fn save(s: &AppState, p: &ProviderConfig, new: bool) -> ApiResult<()> {
    let kind = if p.kind == ProviderKind::Anthropic { "anthropic" } else { "openai-compatible" };
    let sql = if new {
        "INSERT INTO custom_providers (name, kind, base_url, local, api_key_env, api_key_file, max_tokens, context_window, updated_at, id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?9)"
    } else {
        "UPDATE custom_providers SET name = ?1, kind = ?2, base_url = ?3, local = ?4, api_key_env = ?5, api_key_file = ?6,
         max_tokens = ?7, context_window = ?8, updated_at = ?9 WHERE id = ?10"
    };
    sqlx::query(sql)
        .bind(&p.name)
        .bind(kind)
        .bind(&p.base_url)
        .bind(p.local)
        .bind(&p.api_key_env)
        .bind(&p.api_key_file)
        .bind(p.max_tokens)
        .bind(p.context_window)
        .bind(util::now())
        .bind(&p.id)
        .execute(&s.db)
        .await?;
    load(s).await;
    Ok(())
}

/// POST /api/admin/providers: adds one.
pub async fn create(State(s): State<AppState>, Extension(u): Extension<User>, Json(f): Json<Form>) -> ApiResult<(StatusCode, Json<Value>)> {
    crate::admin::require_admin(&s, &u).await?;
    let p = check(&s, &new_id(&s, &f.name), &f)?;
    save(&s, &p, true).await?;
    Ok((StatusCode::CREATED, Json(shown(&p))))
}

/// PUT /api/admin/providers/{id}: changes an added one (the config file's stay as they are).
pub async fn update(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Json(f): Json<Form>) -> ApiResult<Json<Value>> {
    crate::admin::require_admin(&s, &u).await?;
    if s.config.provider(&id).is_some() {
        return Err(ApiError::BadRequest("This provider is set in kompanion.toml; change it there.".into()));
    }
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM custom_providers WHERE id = ?").bind(&id).fetch_optional(&s.db).await?;
    if exists.is_none() {
        return Err(ApiError::NotFound);
    }
    let p = check(&s, &id, &f)?;
    save(&s, &p, false).await?;
    Ok(Json(shown(&p)))
}

/// DELETE /api/admin/providers/{id}: removes an added one no role uses.
pub async fn delete(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    crate::admin::require_admin(&s, &u).await?;
    let roles: Vec<String> = sqlx::query_scalar("SELECT DISTINCT role FROM user_roles WHERE provider_id = ? ORDER BY role").bind(&id).fetch_all(&s.db).await?;
    if !roles.is_empty() {
        return Err(ApiError::BadRequest(format!("Roles still use it ({}): pick another model for them first.", roles.join(", "))));
    }
    let gone = sqlx::query("DELETE FROM custom_providers WHERE id = ?").bind(&id).execute(&s.db).await?;
    if gone.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    load(&s).await;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
#[path = "providers_admin_tests.rs"]
mod tests;
