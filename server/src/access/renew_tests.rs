use super::*;

#[test]
fn a_renewal_revokes_first_only_when_the_runner_would_keep_the_old_expiry() {
    // The runner keeps the later expiry and keeps an expiry when the new grant has none.
    assert!(needs_revoke(Some("2026-10-07T12:00:00Z"), None));
    assert!(needs_revoke(Some("2026-10-07T12:00:00Z"), Some("2026-10-07T11:00:00Z")));
    assert!(!needs_revoke(Some("2026-10-07T12:00:00Z"), Some("2026-10-08T12:00:00Z")));
    assert!(!needs_revoke(None, Some("2026-10-08T12:00:00Z")));
    assert!(!needs_revoke(None, None));
}
