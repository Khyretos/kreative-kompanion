use super::*;
use crate::grants::Grants;

const NOW: &str = "2026-10-03T20:00:00Z";

/// A temp dir with "inside/" granted `rights` and a sibling "outside/".
fn setup(tag: u32, rights: &str) -> (std::path::PathBuf, Grants) {
    let base = std::env::temp_dir().join(format!("kk-tools-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("inside/sub")).unwrap();
    std::fs::create_dir_all(base.join("outside")).unwrap();
    let base = std::fs::canonicalize(&base).unwrap();
    std::fs::write(base.join("inside/a.txt"), "hello").unwrap();
    std::fs::write(base.join("outside/b.txt"), "secret").unwrap();
    let target = serde_json::to_string(&base.join("inside").display().to_string()).unwrap();
    let file = base.join("grants.json");
    std::fs::write(
        &file,
        format!(r#"[{{"target":{target},"rights":[{rights}],"granted_by":"k","granted_at":"2026-10-03T18:00:00Z"}}]"#,),
    )
    .unwrap();
    let g = Grants::load(&file).unwrap();
    (base, g)
}

fn p(base: &std::path::Path, rel: &str) -> String {
    base.join(rel).display().to_string()
}

#[test]
fn read_inside_outside_and_escape() {
    let (b, g) = setup(1, r#""read""#);
    let ok = run(&g, &Tool::ReadFile { path: p(&b, "inside/a.txt"), offset: None, length: None }, NOW);
    assert!(ok.ok && ok.output.contains("hello"), "{}", ok.output);
    assert!(!run(&g, &Tool::ReadFile { path: p(&b, "outside/b.txt"), offset: None, length: None }, NOW).ok);
    assert!(!run(&g, &Tool::ReadFile { path: p(&b, "inside/../outside/b.txt"), offset: None, length: None }, NOW).ok);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn read_a_slice_of_a_file() {
    let (b, g) = setup(20, r#""read""#);
    let slice = run(&g, &Tool::ReadFile { path: p(&b, "inside/a.txt"), offset: Some(1), length: Some(3) }, NOW);
    assert!(slice.ok && slice.output.starts_with("ell\n"), "{}", slice.output);
    assert!(slice.output.contains("more follows, continue with offset 4"), "{}", slice.output);
    let tail = run(&g, &Tool::ReadFile { path: p(&b, "inside/a.txt"), offset: Some(3), length: None }, NOW);
    assert!(tail.ok && tail.output.starts_with("lo\n") && tail.output.contains("end of file"), "{}", tail.output);
    let past = run(&g, &Tool::ReadFile { path: p(&b, "inside/a.txt"), offset: Some(99), length: Some(1) }, NOW);
    assert!(!past.ok && past.output.contains("past the end"), "{}", past.output);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn a_slice_still_needs_the_read_grant() {
    let (b, g) = setup(21, r#""read""#);
    let out = run(&g, &Tool::ReadFile { path: p(&b, "outside/b.txt"), offset: Some(0), length: Some(2) }, NOW);
    assert!(!out.ok && out.output.contains("not granted"), "{}", out.output);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn a_big_file_needs_a_slice_and_a_slice_reads_it() {
    let (b, g) = setup(22, r#""read""#);
    let big = b.join("inside/big.txt");
    std::fs::write(&big, "x".repeat(300 * 1024)).unwrap();
    let whole = run(&g, &Tool::ReadFile { path: p(&b, "inside/big.txt"), offset: None, length: None }, NOW);
    assert!(!whole.ok && whole.output.contains("offset and length"), "{}", whole.output);
    let part = run(&g, &Tool::ReadFile { path: p(&b, "inside/big.txt"), offset: Some(299 * 1024), length: Some(10) }, NOW);
    assert!(part.ok && part.output.starts_with("xxxxxxxxxx\n"), "{}", part.output);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn write_needs_write() {
    let (b, g) = setup(2, r#""read""#);
    assert!(!run(&g, &Tool::WriteFile { path: p(&b, "inside/new.txt"), content: "x".into(), append: false, offset: None }, NOW).ok);
    let (b2, g2) = setup(3, r#""read","write""#);
    let w = run(&g2, &Tool::WriteFile { path: p(&b2, "inside/new.txt"), content: "x".into(), append: false, offset: None }, NOW);
    assert!(w.ok, "{}", w.output);
    assert_eq!(std::fs::read_to_string(b2.join("inside/new.txt")).unwrap(), "x");
    let _ = std::fs::remove_dir_all(&b);
    let _ = std::fs::remove_dir_all(&b2);
}

#[test]
fn a_big_file_is_written_in_parts_and_a_part_sent_again_lands_in_place() {
    let (b, g) = setup(23, r#""read","write""#);
    let w = |content: &str, append: bool, offset: Option<u64>| run(&g, &Tool::WriteFile { path: p(&b, "inside/big.txt"), content: content.into(), append, offset }, NOW);
    // A part at an offset needs the file to be there.
    let none = w("x", false, Some(0));
    assert!(!none.ok && none.output.contains("write the first part without offset"), "{}", none.output);
    assert!(w("part1|", false, None).ok);
    let second = w("part2|", true, None);
    assert!(second.ok && second.output.contains("the file is 12 bytes now"), "{}", second.output);
    // Sent again at the offset the first part ended: replaces what was there, nothing doubled.
    let again = w("PART2|", false, Some(6));
    assert!(again.ok && again.output.contains("at byte 6"), "{}", again.output);
    assert!(w("part3", true, None).ok);
    assert_eq!(std::fs::read_to_string(b.join("inside/big.txt")).unwrap(), "part1|PART2|part3");
    let past = w("x", false, Some(99));
    assert!(!past.ok && past.output.contains("past the end"), "{}", past.output);
    let both = w("x", true, Some(1));
    assert!(!both.ok && both.output.contains("not both"), "{}", both.output);
    // Append makes the file when it is missing; parts still need the write grant.
    assert!(run(&g, &Tool::WriteFile { path: p(&b, "inside/fresh.txt"), content: "a".into(), append: true, offset: None }, NOW).ok);
    let out = run(&g, &Tool::WriteFile { path: p(&b, "outside/x.txt"), content: "a".into(), append: true, offset: None }, NOW);
    assert!(!out.ok && out.output.contains("not granted"), "{}", out.output);
    // JSON from the server: the old shape still works.
    let t: Tool = serde_json::from_str(r#"{"tool":"write_file","path":"/a","content":"x"}"#).unwrap();
    assert!(matches!(t, Tool::WriteFile { append: false, offset: None, .. }));
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn list_dir_marks_folders() {
    let (b, g) = setup(4, r#""read""#);
    let l = run(&g, &Tool::ListDir { path: p(&b, "inside") }, NOW);
    assert!(l.ok && l.output.contains("sub/") && l.output.contains("a.txt"), "{}", l.output);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn shell_needs_shell_and_reports_exit() {
    let (b, g) = setup(5, r#""read""#);
    assert!(!run(&g, &Tool::Shell { cwd: p(&b, "inside"), command: "echo hi".into() }, NOW).ok);
    let (b2, g2) = setup(6, r#""shell""#);
    let ok = run(&g2, &Tool::Shell { cwd: p(&b2, "inside"), command: "echo hi".into() }, NOW);
    assert!(ok.ok && ok.output.contains("hi") && ok.output.ends_with("exit: 0"), "{}", ok.output);
    let bad = run(&g2, &Tool::Shell { cwd: p(&b2, "inside"), command: "exit 3".into() }, NOW);
    assert!(!bad.ok && bad.output.ends_with("exit: 3"), "{}", bad.output);
    let _ = std::fs::remove_dir_all(&b);
    let _ = std::fs::remove_dir_all(&b2);
}
