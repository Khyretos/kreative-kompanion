//! KS-01: Kreative Studio calls the Studio API with its service token and the studio user's e-mail
//! (X-Studio-User); this maps the e-mail to a Kompanion user for the normal handlers.
use crate::{AppState, auth::User, error::{ApiError, ApiResult}};

pub const STUDIO_PREFIX: &str = "studio-";
pub const USER_HEADER: &str = "x-studio-user";

pub fn studio_only_id(email: &str) -> String {
    let trimmed_lower = email.trim().to_lowercase();
    let hash = crate::util::sha256_hex(&trimmed_lower);
    format!("{}{}", STUDIO_PREFIX, &hash[..16])
}

pub fn valid_email(email: &str) -> bool {
    let s = email.trim();
    if s.is_empty() || s.len() > 254 { return false; }
    let mut at_count = 0;
    for c in s.chars() {
        if c == '@' { at_count += 1; }
        if c.is_whitespace() || c == '/' { return false; }
    }
    at_count == 1 && s.contains('@') && s.split('@').next().map(|s| !s.is_empty()).unwrap_or(false) && s.split('@').last().map(|s| !s.is_empty()).unwrap_or(false)
}

pub async fn user_for(db: &sqlx::SqlitePool, email: &str) -> ApiResult<User> {
    if !valid_email(email) {
        return Err(ApiError::BadRequest("X-Studio-User must be an e-mail address.".into()));
    }
    
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT id, name FROM users WHERE lower(email) = lower(?) LIMIT 1"
    )
    .bind(email.trim())
    .fetch_optional(db)
    .await
    .map_err(anyhow::Error::from)?;
    
    match row {
        Some((id, name)) => Ok(User { id, name }),
        None => Ok(User { 
            id: studio_only_id(email), 
            name: email.trim().to_lowercase() 
        }),
    }
}

pub async fn as_user(s: &AppState, req: &mut axum::extract::Request) -> ApiResult<()> {
    super::target::service_guard(s, req.headers()).await?;
    
    let email = req
        .headers()
        .get(USER_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("X-Studio-User is missing.".into()))?
        .to_string();
    
    let u = user_for(&s.db, &email).await?;
    req.extensions_mut().insert(u);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_ids_ignore_case_and_spaces() {
        let a = studio_only_id("Ana@Example.com");
        assert_eq!(a, studio_only_id("  ana@example.com "));
        assert!(a.starts_with("studio-"));
        assert_eq!(a.len(), 23);
        assert_ne!(a, studio_only_id("bob@example.com"));
    }

    #[test]
    fn emails_are_checked() {
        assert!(valid_email("ana@example.com"));
        assert!(!valid_email(""));
        assert!(!valid_email("ana"));
        assert!(!valid_email("@example.com"));
        assert!(!valid_email("ana@"));
        assert!(!valid_email("a@b@c"));
        assert!(!valid_email("an a@example.com"));
        assert!(!valid_email("../x@example.com"));
    }
}
