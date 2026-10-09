//! The server address the user types, cleaned into the URL the app loads.

/// Cleans a server address the user typed into the base URL the app loads.
pub fn normalize(input: &str) -> Result<String, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter the server address.");
    }

    let (scheme, rest) = match trimmed.find("://") {
        Some(idx) => (trimmed[..idx].to_lowercase(), &trimmed[idx + 3..]),
        None => ("https".to_owned(), trimmed),
    };
    if scheme != "http" && scheme != "https" {
        return Err("Use an http or https address.");
    }

    let first_sep = rest.find(['/', '?', '#']);
    let host_part: String = if let Some(idx) = first_sep {
        rest[..idx].to_string()
    } else {
        rest.to_string()
    };

    if host_part.is_empty() || host_part.contains('@') || host_part.contains(' ') {
        return Err("That is not a server address.");
    }

    let host_lower = host_part.to_lowercase();

    let remainder = if let Some(idx) = first_sep {
        &rest[idx..]
    } else {
        ""
    };

    let path_start = remainder.find(['?', '#']);
    let path: String = if let Some(idx) = path_start {
        remainder[..idx].trim_end_matches('/').to_string()
    } else {
        remainder.trim_end_matches('/').to_string()
    };

    Ok(format!("{}://{}{}", scheme, host_lower, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_host() {
        assert_eq!(normalize("kompanion.example.com").unwrap(), "https://kompanion.example.com");
    }

    #[test]
    fn test_case_and_whitespace() {
        assert_eq!(normalize(" HTTPS://Kompanion.Example.com/ ").unwrap(), "https://kompanion.example.com");
    }

    #[test]
    fn test_http_with_trailing_slash() {
        assert_eq!(normalize("http://192.168.1.5:8095/").unwrap(), "http://192.168.1.5:8095");
    }

    #[test]
    fn test_path_query_hash() {
        assert_eq!(normalize("https://example.com/kompanion/?x=1#a").unwrap(), "https://example.com/kompanion");
    }

    #[test]
    fn test_invalid_scheme() {
        assert_eq!(normalize("ftp://x"), Err("Use an http or https address."));
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(normalize(""), Err("Enter the server address."));
    }

    #[test]
    fn test_user_at_host() {
        assert_eq!(normalize("https://user@host"), Err("That is not a server address."));
    }

    #[test]
    fn test_empty_host() {
        assert_eq!(normalize("https:///path"), Err("That is not a server address."));
    }
}
