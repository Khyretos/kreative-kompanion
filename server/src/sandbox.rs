//! ARTIFACTS: the preview frame for HTML the model writes. The page itself is the web app's
//! `sandbox.html`; this route serves it with a policy of its own instead of the app's: a sandboxed,
//! opaque origin (no cookies, no app storage, no API), inline scripts and styles allowed so a
//! preview can run, and no network at all, so a preview can't send anything anywhere.
use axum::{extract::State, http::header, response::IntoResponse};

use crate::AppState;

pub const SANDBOX_CSP: &str = "sandbox allow-scripts; default-src 'none'; script-src 'unsafe-inline'; \
style-src 'unsafe-inline'; img-src data: blob:; font-src data:; media-src data: blob:; frame-ancestors 'self'";

/// The headers for the preview frame, apart from the content type.
pub fn headers() -> [(header::HeaderName, &'static str); 3] {
    [
        (header::CONTENT_SECURITY_POLICY, SANDBOX_CSP),
        (header::CACHE_CONTROL, "no-cache"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ]
}

pub async fn page(State(s): State<AppState>) -> axum::response::Response {
    match tokio::fs::read(s.config.web_dir.join("sandbox.html")).await {
        Ok(bytes) => (headers(), [(header::CONTENT_TYPE, "text/html; charset=utf-8")], bytes).into_response(),
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "no preview frame").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_frame_is_sandboxed_and_offline() {
        assert!(SANDBOX_CSP.starts_with("sandbox allow-scripts;"), "opaque origin, scripts only");
        assert!(!SANDBOX_CSP.contains("allow-same-origin"), "never the app's origin");
        assert!(SANDBOX_CSP.contains("default-src 'none'"), "no network by default");
        assert!(!SANDBOX_CSP.contains("connect-src"), "fetch and XHR fall back to none");
        assert!(SANDBOX_CSP.contains("frame-ancestors 'self'"), "only the app may frame it");
        let h = headers();
        assert_eq!(h[0].1, SANDBOX_CSP);
    }
}
