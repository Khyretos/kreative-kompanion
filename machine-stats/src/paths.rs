//! Folder paths from both worlds (RUNNER-WIN-2): `/home/k/app` on Linux, `C:\Users\k\app` on Windows.
//! Plain text rules, so the server (which never touches those disks) and the runner use the same ones.

/// True for an absolute path on unix (`/x`) or Windows (`C:\x`, `c:/x`, `\\server\share`).
pub fn is_absolute_target(t: &str) -> bool {
    if t.starts_with('/') || t.starts_with("\\\\") {
        return true;
    }
    let mut chars = t.chars();
    matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(d), Some(':'), Some('/' | '\\')) if d.is_ascii_alphabetic()
    )
}

/// A path without the Windows verbatim prefix that canonicalize adds (`\\?\UNC\srv\sh` becomes `\\srv\sh`).
pub fn plain_path_str(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{rest}")
    } else if let Some(rest) = p.strip_prefix("\\\\?\\") {
        rest.to_string()
    } else {
        p.to_string()
    }
}

/// True for a target that is not a unix path (`C:\x`, `\\srv\sh`): compared like Windows paths.
pub fn looks_windows(s: &str) -> bool {
    !s.starts_with('/')
}

/// True when `path` is the folder `target` or inside it, compared component-wise as text: on Windows
/// (`windows` true) case-insensitive and with `/` and `\` equal, on unix exact. A trailing separator on the
/// target is ignored, the target `/` matches every unix path.
pub fn starts_with_target(windows: bool, path: &str, target: &str) -> bool {
    let norm = |s: &str| -> String {
        let plain = plain_path_str(s);
        let plain = if windows { plain.replace('\\', "/").to_lowercase() } else { plain };
        if plain == "/" { plain } else { plain.trim_end_matches('/').to_string() }
    };
    let (p, t) = (norm(path), norm(target));
    (t == "/" && p.starts_with('/')) || p == t || p.starts_with(&format!("{t}/"))
}

/// True for an absolute folder without `..` parts, with either kind of separator.
pub fn folder_ok(p: &str) -> bool {
    is_absolute_target(p) && !p.split(['/', '\\']).any(|c| c == "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_targets_on_both_systems() {
        assert!(is_absolute_target("/home/k"));
        assert!(is_absolute_target("C:\\Users\\k"));
        assert!(is_absolute_target("c:/Users/k"));
        assert!(is_absolute_target("\\\\server\\share\\x"));
        assert!(!is_absolute_target("relative/dir"));
        assert!(!is_absolute_target("C:"));
        assert!(!is_absolute_target("C:rel"));
        assert!(!is_absolute_target(""));
    }

    #[test]
    fn verbatim_prefix_is_stripped() {
        assert_eq!(plain_path_str("\\\\?\\C:\\Users\\k"), "C:\\Users\\k");
        assert_eq!(plain_path_str("\\\\?\\UNC\\srv\\sh\\x"), "\\\\srv\\sh\\x");
        assert_eq!(plain_path_str("/home/k"), "/home/k");
        assert_eq!(plain_path_str("C:\\x"), "C:\\x");
    }

    #[test]
    fn target_prefix_match_per_system() {
        assert!(starts_with_target(true, "C:\\Users\\Kees\\proj\\a.txt", "c:/users/kees"));
        assert!(!starts_with_target(true, "C:\\Users\\Keesje\\a", "C:\\Users\\Kees"));
        assert!(starts_with_target(true, "C:\\Users\\Kees", "C:\\Users\\Kees\\"));
        assert!(starts_with_target(true, "\\\\?\\C:\\Users\\Kees\\a", "C:\\Users\\Kees"));
        assert!(starts_with_target(false, "/home/k/p", "/home/k"));
        assert!(starts_with_target(false, "/home/k", "/home/k/"));
        assert!(!starts_with_target(false, "/home/k2", "/home/k"));
        assert!(!starts_with_target(false, "/Home/k/p", "/home/k"));
        assert!(starts_with_target(false, "/anything", "/"));
    }

    #[test]
    fn a_folder_is_absolute_and_has_no_dotdot() {
        assert!(folder_ok("/home/k/app"));
        assert!(folder_ok("C:\\Users\\k\\app"));
        assert!(folder_ok("c:/Users/k/app"));
        assert!(!folder_ok("C:\\Users\\..\\x"));
        assert!(!folder_ok("/a/../b"));
        assert!(!folder_ok("relative"));
        assert!(!folder_ok(""));
    }

    #[test]
    fn windows_style_means_not_starting_with_a_slash() {
        assert!(looks_windows("C:\\x"));
        assert!(looks_windows("\\\\srv\\sh"));
        assert!(!looks_windows("/home/k"));
    }
}
