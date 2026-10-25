use std::path::Path;

/// Returns the list of files named by a shell command string.
/// 
/// Splits on whitespace, filters out flags, operators, and non-file tokens,
/// stripping quotes and leading "./". Returns unique files in order.
pub fn check_files(cmd: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for token in cmd.split_whitespace() {
        // Strip surrounding single or double quotes (only if length >= 2)
        let token = if token.len() >= 2 {
            if token.starts_with('"') && token.ends_with('"') {
                &token[1..token.len()-1]
            } else if token.starts_with('\'') && token.ends_with('\'') {
                &token[1..token.len()-1]
            } else {
                token
            }
        } else {
            token
        };

        // Skip flags, redirections, and variable assignments
        if token.contains('=') || token.contains('>') || token.contains('<') {
            continue;
        }

        // Skip flags
        if token.starts_with('-') {
            continue;
        }

        // Skip "." and ".."
        if token == "." || token == ".." {
            continue;
        }

        // Skip shell operators and commands that don't look like files.
        // A file token must contain '.' or '/' after the first character.
        // This excludes operators like |, &, ;, >, <, (, ), {, }, [, ], ` 
        // and commands like cd, ls, python3 which lack these characters.
        if !token.contains('.') && !token.contains('/') {
            continue;
        }

        // Strip leading "./"
        let token = token.strip_prefix("./").unwrap_or(token);

        // Deduplicate preserving order
        if seen.insert(token.to_string()) {
            result.push(token.to_string());
        }
    }

    result
}

/// Checks if a given path is protected based on naming conventions or extra rules.
/// 
/// Returns true if the file matches test_* patterns, *_test patterns,
/// ends with .spec.ts or .test.ts, or resides in a 'tests' directory,
/// or matches an entry in the extra list.
pub fn is_protected(path: &str, extra: &[String]) -> bool {
    // Normalize path: strip leading "./" if present for comparison logic
    let normalized_path = path.strip_prefix("./").unwrap_or(path);
    
    // Check extra rules first
    for entry in extra {
        let entry_normalized = entry.strip_prefix("./").unwrap_or(entry);
        if entry_normalized == normalized_path {
            return true;
        }
        if normalized_path.ends_with(&format!("/{}", entry_normalized)) {
            return true;
        }
    }

    // Check directory components for "tests"
    let parts: Vec<&str> = normalized_path.split('/').collect();
    if parts.iter().any(|p| *p == "tests") {
        return true;
    }

    // Check filename patterns
    let stem = Path::new(normalized_path).file_name().and_then(|n| n.to_str()).unwrap_or("");
    
    if stem.starts_with("test_") && stem.ends_with(".py") {
        return true;
    }
    if stem.ends_with("_test.py") || stem.ends_with("_test.go") {
        return true;
    }
    if stem.ends_with(".spec.ts") || stem.ends_with(".test.ts") {
        return true;
    }

    false
}

/// A protected file this run may still change: one the task's description asks to change.
pub fn guarded(path: &str, extra: &[String], allowed: &[String]) -> bool {
    is_protected(path, extra) && !allowed.iter().any(|a| same_file(path, a))
}

fn same_file(path: &str, name: &str) -> bool {
    let (p, n) = (path.strip_prefix("./").unwrap_or(path), name.strip_prefix("./").unwrap_or(name));
    p == n || p.ends_with(&format!("/{n}")) || n.ends_with(&format!("/{p}"))
}

/// DOGFOOD: the test files a task's description asks to change ("2. Add a test for a quoted # to
/// test_big.py."): a sentence that names the file and says add, write, extend, update, change, edit
/// or create. A file named only to run it ("make tests/test_x.py pass") stays protected.
pub fn named_in(description: &str, extra: &[String]) -> Vec<String> {
    const VERBS: [&str; 7] = ["add", "write", "extend", "update", "change", "edit", "create"];
    let mut out = Vec::new();
    let clean = |w: &str| w.trim_matches(|c: char| "`'\",:;()[]*".contains(c)).trim_end_matches('.').to_string();
    for sentence in description.lines().flat_map(|l| l.split(". ")).flat_map(|l| l.split("; ")) {
        let words: Vec<String> = sentence.split_whitespace().map(|w| clean(w).to_lowercase()).collect();
        if !words.iter().any(|w| VERBS.contains(&w.as_str())) {
            continue;
        }
        for w in sentence.split_whitespace() {
            let f = clean(w);
            if (f.contains('.') || f.contains('/')) && is_protected(&f, extra) && !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
}

/// RUN-01: the files a shell command writes or removes, so a step can't get round the test protection
/// with the shell: redirect targets (> >> 2> &>), tee's files, sed/perl -i's files, cp/mv/install
/// destinations, files rm, truncate, touch or shred name, and dd's of=. A guard, not a shell parser.
pub fn shell_writes(cmd: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |f: &str| {
        let f = f.trim_matches(|c| c == '"' || c == '\'');
        if !f.is_empty() && !f.starts_with('&') && f != "/dev/null" && !out.iter().any(|o| o == f) {
            out.push(f.to_string());
        }
    };
    let flat = cmd.replace("&&", ";").replace("||", ";").replace('|', ";").replace('\n', ";");
    for seg in flat.split(';') {
        let toks: Vec<&str> = seg.split_whitespace().collect();
        // Redirections anywhere in the segment.
        let mut i = 0;
        while i < toks.len() {
            let t = toks[i];
            if matches!(t, ">" | ">>" | "1>" | "2>" | "&>" | "1>>" | "2>>" | "&>>") {
                if let Some(n) = toks.get(i + 1) { add(n); }
                i += 2;
                continue;
            }
            if let Some(pos) = t.rfind('>') && !t.starts_with("<<") && pos + 1 < t.len() {
                add(&t[pos + 1..]);
            }
            i += 1;
        }
        // The command word: skip VAR=value and sudo.
        let words: Vec<&str> = toks.iter().copied().skip_while(|w| w.contains('=') && !w.starts_with('-') || *w == "sudo").collect();
        let Some((&cmd_word, rest)) = words.split_first() else { continue };
        let name = cmd_word.rsplit('/').next().unwrap_or(cmd_word);
        let args: Vec<&str> = rest.iter().copied().take_while(|w| !w.contains('>') && !w.starts_with("<<")).collect();
        let plain: Vec<&str> = args.iter().copied().filter(|a| !a.starts_with('-')).collect();
        match name {
            "tee" | "rm" | "truncate" | "touch" | "shred" | "unlink" => plain.iter().for_each(|f| add(f)),
            "sed" | "perl" if args.iter().any(|a| a.starts_with("-i") || (a.starts_with("-") && !a.starts_with("--") && a.contains('i'))) => {
                // Scripts come after -e; without -e the first plain word is the script.
                let scripted = args.iter().any(|a| *a == "-e" || *a == "--expression");
                let scripts: Vec<&str> = args.windows(2).filter(|w| w[0] == "-e" || w[0] == "--expression").map(|w| w[1]).collect();
                plain.iter().filter(|f| !scripts.contains(f)).skip(if scripted { 0 } else { 1 }).for_each(|f| add(f));
            }
            "cp" | "mv" | "install" | "ln" => {
                if let Some((dest, srcs)) = plain.split_last() {
                    add(dest);
                    // Into a folder (a trailing / or several sources): the files land there under their own names.
                    for s in srcs.iter().filter(|_| dest.ends_with('/') || srcs.len() > 1) {
                        add(&format!("{}/{}", dest.trim_end_matches('/'), s.rsplit('/').next().unwrap_or(s)));
                    }
                }
            }
            "dd" => args.iter().filter_map(|a| a.strip_prefix("of=")).for_each(|f| add(f)),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_writes_finds_what_a_command_changes() {
        assert_eq!(shell_writes("cat >> test_big.py << 'EOF'\n    def test_x(self): pass\nEOF"), vec!["test_big.py"]);
        assert_eq!(shell_writes("cat > tests/a.py"), vec!["tests/a.py"]);
        assert_eq!(shell_writes("echo x>test_a.py && ls"), vec!["test_a.py"]);
        assert_eq!(shell_writes("python3 -m unittest 2>/dev/null | tee out.log"), vec!["out.log"]);
        assert_eq!(shell_writes("sed -i 's/a/b/' test_a.py big.py"), vec!["test_a.py", "big.py"]);
        assert_eq!(shell_writes("sed -i -e 's/a/b/' test_a.py"), vec!["test_a.py"]);
        assert_eq!(shell_writes("perl -pi -e 's/a/b/' t/x_test.go"), vec!["t/x_test.go"]);
        assert_eq!(shell_writes("cp /tmp/new.py tests/"), vec!["tests/", "tests/new.py"]);
        assert_eq!(shell_writes("mv a.py test_a.py"), vec!["test_a.py"]);
        assert_eq!(shell_writes("rm -f test_a.py; touch x.txt"), vec!["test_a.py", "x.txt"]);
        assert_eq!(shell_writes("dd if=/dev/zero of=tests/blob bs=1 count=1"), vec!["tests/blob"]);
        assert_eq!(shell_writes("FOO=1 sudo truncate -s0 test_a.py"), vec!["test_a.py"]);
        // Reading never counts.
        assert!(shell_writes("cat test_big.py && grep -n parse big.py && sed -n '1,5p' test_a.py && python3 -m unittest 2>&1").is_empty());
    }

    #[test]
    fn a_test_file_the_task_asks_to_change_may_change() {
        let d = "**Goal:** keep a quoted #.\n\n**Steps**\n1. Fix parse_line in big.py.\n2. Add a test for a quoted # to test_big.py.\n\n**Done when:** python3 -m unittest passes.";
        assert_eq!(named_in(d, &[]), vec!["test_big.py"]);
        assert_eq!(named_in("Write tests in `tests/test_cfg.py` for the parser.", &[]), vec!["tests/test_cfg.py"]);
        // Named only to run or pass: still protected.
        assert!(named_in("Make tests/test_parser.py pass. Fix the parser in src/parse.py.", &[]).is_empty());
        assert!(named_in("Add initials(name) to names.py so that python3 -m unittest passes.", &[]).is_empty());
        assert!(guarded("/w/test_x.py", &[], &[]));
        assert!(!guarded("/w/test_big.py", &[], &["test_big.py".into()]));
        assert!(guarded("/w/test_other.py", &[], &["test_big.py".into()]));
    }

    #[test]
    fn test_check_files_example() {
        let cmd = "cd app && python3 -m pytest test_names.py ./check.sh";
        let files = check_files(cmd);
        assert_eq!(files, vec!["test_names.py", "check.sh"]);
    }

    #[test]
    fn test_is_protected_absolute_py() {
        assert!(is_protected("/home/u/proj/test_names.py", &[]));
    }

    #[test]
    fn test_is_protected_relative_py() {
        assert!(is_protected("proj/test_names.py", &[]));
    }

    #[test]
    fn test_is_protected_not_regular_py() {
        assert!(!is_protected("/home/u/proj/names.py", &[]));
    }

    #[test]
    fn test_is_protected_json_in_tests_dir() {
        assert!(is_protected("/p/tests/data.json", &[]));
    }

    #[test]
    fn test_is_protected_shell_in_extra() {
        assert!(is_protected("/p/src/check.sh", &["check.sh".to_string()]));
    }

    #[test]
    fn test_is_protected_not_in_extra() {
        assert!(!is_protected("/p/src/recheck.sh", &["check.sh".to_string()]));
    }
}
