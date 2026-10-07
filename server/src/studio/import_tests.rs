use super::*;
use std::path::Path;

#[test]
fn ids_and_types_are_checked() {
    assert!(valid_id("f720947d-c295-4343-9d92-cb29244e014b"));
    assert!(!valid_id(""));
    assert!(!valid_id("../x"));
    assert!(!valid_id(&"a".repeat(65)));
    assert!(valid_type("vn-portrait"));
    assert!(valid_type("sfx"));
    assert!(!valid_type("Vn"));
    assert!(!valid_type("a/b"));
    assert!(!valid_type(""));
}

#[test]
fn copies_get_numbered_names() {
    assert_eq!(target_name("abc", 0, Path::new("/g/images/x/a_00001_.PNG")), "abc-1.png");
    assert_eq!(target_name("abc", 1, Path::new("/g/audio/b.wav")), "abc-2.wav");
    assert_eq!(target_name("abc", 2, Path::new("/g/audio/noext")), "abc-3.bin");
}

#[test]
fn created_ms_becomes_rfc3339() {
    assert_eq!(started_at(0), "1970-01-01T00:00:00Z");
    assert_eq!(started_at(1_791_178_179_331), "2026-10-05T05:29:39.331Z");
}

#[test]
fn sources_must_be_shared_but_not_kompanions_own() {
    let shared = Path::new("/generated");
    let out = Path::new("/generated/kompanion");
    assert!(source_ok(Path::new("/generated/images/u/oc/a.png"), shared, out));
    assert!(!source_ok(Path::new("/generated/kompanion/users/x/a.png"), shared, out));
    assert!(!source_ok(Path::new("/etc/passwd"), shared, out));
}
