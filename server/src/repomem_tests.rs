//! KG-06 tests, run against a real git fixture: the commands answer small, correct slices and a hostile
//! query stays a search word.
use super::*;
use std::process::Command;

fn fixture() -> Option<std::path::PathBuf> {
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("git missing: repomem fixture tests skipped");
        return None;
    }
    let dir = std::env::temp_dir().join(format!("kk-repomem-{}", crate::util::new_id()));
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    for i in 0..30 {
        std::fs::create_dir_all(dir.join(format!("src/mod{}", i % 3))).unwrap();
        let body: String = (0..120).map(|n| format!("// filler line {n} of file {i}\n")).collect();
        std::fs::write(dir.join(format!("src/mod{}/f{i}.rs", i % 3)), format!("pub fn helper_{i}() {{}}\n{body}")).unwrap();
    }
    std::fs::write(dir.join("src/mod0/text.rs"), "pub async fn char_count(s: &str) -> usize {\n    s.len()\n}\nstruct Counter;\n").unwrap();
    std::fs::write(dir.join("CLAUDE.md"), "Conventions: no unwrap in handlers.\n").unwrap();
    std::fs::write(dir.join("docs/lessons-learned.md"), "- 12: never use cargo fmt here\n- 13: tests first\n").unwrap();
    for args in [vec!["init", "-q"], vec!["add", "-A"]] {
        assert!(Command::new("git").current_dir(&dir).args(&args).status().unwrap().success());
    }
    Some(dir)
}

fn run(dir: &std::path::Path, tool: &str, args: Value) -> String {
    let cmd = command(tool, &args).expect("a command");
    let out = Command::new("sh").current_dir(dir).arg("-c").arg(cmd).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn the_map_is_a_small_slice_of_the_repo_and_carries_the_conventions() {
    let Some(dir) = fixture() else { return };
    let map = trim(&run(&dir, "repo_map", json!({})));
    let repo_bytes: u64 = walk(&dir.join("src"));
    assert!(map.contains("src/mod0/"), "{map}");
    assert!(map.contains("no unwrap in handlers"));
    assert!((map.len() as u64) * 10 < repo_bytes, "map {} bytes vs source {repo_bytes}", map.len());
}

fn walk(dir: &std::path::Path) -> u64 {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            if p.is_dir() { walk(&p) } else { std::fs::metadata(&p).unwrap().len() }
        })
        .sum()
}

#[test]
fn symbols_find_the_definition_with_file_and_line() {
    let Some(dir) = fixture() else { return };
    let hit = run(&dir, "repo_symbols", json!({"query": "char_count"}));
    assert!(hit.contains("src/mod0/text.rs:1:pub async fn char_count"), "{hit}");
    assert_eq!(hit.lines().count(), 1);
    assert!(run(&dir, "repo_symbols", json!({"query": "Counter"})).contains("struct Counter"));
    assert!(run(&dir, "repo_symbols", json!({"query": "helper_"})).lines().count() <= 25);
}

#[test]
fn decisions_return_only_the_matching_lines() {
    let Some(dir) = fixture() else { return };
    let hit = run(&dir, "repo_decisions", json!({"query": "cargo fmt"}));
    assert_eq!(hit.trim().lines().count(), 1, "{hit}");
    assert!(hit.contains("never use cargo fmt"));
}

#[test]
fn a_hostile_query_is_only_a_search_word() {
    let Some(dir) = fixture() else { return };
    let q = "x'; touch pwned; echo '";
    let _ = run(&dir, "repo_symbols", json!({"query": q}));
    let _ = run(&dir, "repo_decisions", json!({"query": q}));
    assert!(!dir.join("pwned").exists());
}

#[test]
fn wrong_calls_give_no_command_and_long_answers_are_cut_at_a_line() {
    assert!(command("repo_symbols", &json!({})).is_none());
    assert!(command("repo_symbols", &json!({"query": "  "})).is_none());
    assert!(command("repo_symbols", &json!({"query": "a".repeat(81)})).is_none());
    assert!(command("repo_symbols", &json!({"query": "a\nb"})).is_none());
    assert!(command("shell", &json!({})).is_none());
    let long: String = (0..2000).map(|n| format!("line {n}\n")).collect();
    let cut = trim(&long);
    assert!(cut.len() < MAX_ANSWER + 60 && cut.ends_with("ask a narrower query)"));
    assert_eq!(trim("  "), "(nothing found)");
    assert!(is_tool("repo_map") && !is_tool("shell"));
    assert_eq!(job("repo_map", &json!({}), "/w").unwrap()["cwd"], "/w");
}
