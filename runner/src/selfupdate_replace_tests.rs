use super::*;

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("kk-replace-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn replace_with_new_moves_the_old_file_aside_and_puts_the_new_one_in_place() {
    let d = dir("ok");
    let (target, new) = (d.join("runner.exe"), d.join("runner.exe.new"));
    std::fs::write(&target, "old").unwrap();
    std::fs::write(&new, "new").unwrap();
    replace_with_new(&new, &target).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
    assert_eq!(std::fs::read_to_string(d.join("runner.exe.old")).unwrap(), "old");
    assert!(!new.exists());
}

#[test]
fn replace_with_new_replaces_a_stale_old_file() {
    let d = dir("stale");
    let (target, new) = (d.join("r.exe"), d.join("r.exe.new"));
    std::fs::write(&target, "old").unwrap();
    std::fs::write(&new, "new").unwrap();
    std::fs::write(d.join("r.exe.old"), "older").unwrap();
    replace_with_new(&new, &target).unwrap();
    assert_eq!(std::fs::read_to_string(d.join("r.exe.old")).unwrap(), "old");
}

#[test]
fn replace_with_new_puts_the_old_file_back_when_the_new_one_is_missing() {
    let d = dir("missing");
    let (target, new) = (d.join("r.exe"), d.join("r.exe.new"));
    std::fs::write(&target, "old").unwrap();
    assert!(replace_with_new(&new, &target).is_err());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
}

#[test]
fn cleanup_old_removes_the_leftover() {
    let d = dir("clean");
    let target = d.join("r.exe");
    std::fs::write(d.join("r.exe.old"), "old").unwrap();
    cleanup_old(&target);
    assert!(!d.join("r.exe.old").exists());
    cleanup_old(&target); // nothing there: no panic
}
