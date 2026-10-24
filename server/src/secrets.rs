//! Token-looking strings (API keys, access tokens) found and hidden before text reaches a model or
//! is stored where it should not be: past chats (HIST-01), facts and the step log (FACTS).

/// Known token prefixes (API keys, access tokens).
const TOKEN_PREFIXES: &[&str] = &["sk-", "ghp_", "gho_", "ghs_", "ghu_", "github_pat_", "glpat-", "xoxb-", "xoxp-", "xoxa-", "hf_", "AKIA", "AIza", "pk_live_", "sk_live_", "rk_live_"];

/// Whether a word looks like a secret: a known token prefix with a long tail, or a long run of
/// letters and digits with upper case, lower case and digits mixed (a git hash, all lower case, is kept).
pub fn looks_secret(word: &str) -> bool {
    let w = word.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'));
    if w.len() < 20 || !w.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return false;
    }
    if TOKEN_PREFIXES.iter().any(|p| w.starts_with(p) && w.len() >= p.len() + 16) {
        return true;
    }
    w.len() >= 32 && w.chars().any(|c| c.is_ascii_uppercase()) && w.chars().any(|c| c.is_ascii_lowercase()) && w.chars().any(|c| c.is_ascii_digit())
}

/// The text with every token-looking word replaced by `[hidden secret]`; spacing is kept.
pub fn mask(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if looks_secret(word) {
            // Keep what surrounds the token (quotes, "=", a trailing comma).
            let start = word.find(|c: char| c.is_ascii_alphanumeric()).unwrap_or(0);
            let end = word.rfind(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-').map_or(word.len(), |i| i + 1);
            out.push_str(&word[..start]);
            out.push_str("[hidden secret]");
            out.push_str(&word[end..]);
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for c in text.chars() {
        if c.is_whitespace() || c == '=' || c == ':' || c == '"' || c == '\'' || c == '`' {
            flush(&mut word, &mut out);
            out.push(c);
        } else {
            word.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// Whether the text holds anything token-looking (a fact may say where a secret lives, never hold it).
pub fn has_secret(text: &str) -> bool {
    text.split(|c: char| c.is_whitespace() || matches!(c, '=' | ':' | '"' | '\'' | '`')).any(looks_secret)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A token-shaped test value built at run time, so no token-like literal sits in the source
    /// (the lint gate's secret scanners read every commit).
    pub fn fake(prefix: &str) -> String {
        format!("{prefix}{}", "Ab3dEf7h".repeat(4))
    }

    #[test]
    fn secrets_are_masked_and_ordinary_words_kept() {
        let (gh, google, bare) = (fake("ghp_"), fake("AIza"), fake(""));
        assert_eq!(mask(&format!("token={gh},")), "token=[hidden secret],");
        assert_eq!(mask(&format!("key: \"{google}\"")), "key: \"[hidden secret]\"");
        assert_eq!(mask(&format!("a {bare} secret")), "a [hidden secret] secret");
        // A git hash (lower case only), a path and normal prose stay as they are.
        let kept = "commit 3073201ad142efb51cade0ffcb4c910a76c90763 in /home/kees/projects/kreative_kompanion_server_src";
        assert_eq!(mask(kept), kept);
        assert_eq!(mask("Internationalization  is long\nnew line"), "Internationalization  is long\nnew line");
        assert!(!looks_secret("sk-short"));
        assert!(looks_secret(&fake("sk-")));
        assert!(has_secret(&format!("Forgejo: {gh}")));
        assert!(!has_secret("Forgejo token: file ~/.forgejo-token"));
    }
}
