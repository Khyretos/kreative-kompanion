//! KG-11: POST /projects/{id}/asset-check. The asset commit (M6-12) calls it before it commits: a batch of files in,
//! the contract issues and the style outliers out. Nothing is stored.
use axum::{Extension, Json, extract::{Path, State}};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState,
    assetcontract::{self, Asset, Profile},
    auth::User,
    devsettings::Settings,
    error::{ApiError, ApiResult},
};

const MAX_FILES: usize = 200;

#[derive(Deserialize)]
pub struct File {
    path: String,
    /// The file's bytes, base64.
    data: String,
}

#[derive(Deserialize)]
pub struct Body {
    files: Vec<File>,
    /// Engine profile; the project's engine setting, then "kk-engine", when missing.
    #[serde(default)]
    engine: String,
}

/// The assets of a request, or what is wrong with it.
pub fn decode(files: &[File]) -> Result<Vec<Asset>, String> {
    if files.is_empty() || files.len() > MAX_FILES {
        return Err(format!("send 1 to {MAX_FILES} files"));
    }
    files
        .iter()
        .map(|f| STANDARD.decode(f.data.trim()).map(|bytes| Asset { path: f.path.clone(), bytes }).map_err(|_| format!("'{}': data is not base64", f.path)))
        .collect()
}

pub async fn check(State(s): State<AppState>, Extension(u): Extension<User>, Path(id): Path<String>, Json(b): Json<Body>) -> ApiResult<Json<Value>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT dev_settings FROM projects WHERE id = ? AND user_id = ?").bind(&id).bind(&u.id).fetch_optional(&s.db).await?;
    let (text,) = row.ok_or(ApiError::NotFound)?;
    let engine = [b.engine.trim().to_string(), Settings::parse(&text).engine, "kk-engine".to_string()].into_iter().find(|e| !e.is_empty()).unwrap_or_default();
    let profile = Profile::named(&engine).ok_or_else(|| ApiError::BadRequest(format!("no asset contract for engine '{engine}'")))?;
    let assets = decode(&b.files).map_err(ApiError::BadRequest)?;
    let mut issues = assetcontract::validate(&profile, &assets);
    issues.extend(assetcontract::style_check(&assets));
    Ok(Json(json!({ "engine": engine, "ok": issues.is_empty(), "checked": assets.len(), "issues": issues })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, data: &str) -> File {
        File { path: path.into(), data: data.into() }
    }

    #[test]
    fn decode_accepts_base64_and_refuses_the_rest() {
        let ok = decode(&[f("a.png", "AQID")]).unwrap();
        assert_eq!(ok[0].bytes, vec![1, 2, 3]);
        assert!(decode(&[f("a.png", "***")]).unwrap_err().contains("a.png"));
        assert!(decode(&[]).is_err());
    }
}
