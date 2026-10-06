//! Which model licences get a warning (LIC-01: a warning only, nothing is refused): OSI/CC0 none, CC-BY without attribution, the rest yes.
pub fn check(licence: &str, attribution: Option<&str>) -> Result<(), String> {
    let lic = licence.trim();
    if lic.is_empty() {
        return Err("unknown: unknown licence, check it before use".to_string());
    }

    let lic_lower = lic.to_lowercase();
    let allowed = [
        "apache-2.0",
        "mit",
        "bsd-2-clause",
        "bsd-3-clause",
        "cc0-1.0",
    ];
    if allowed.contains(&lic_lower.as_str()) {
        return Ok(());
    }

    if lic_lower == "cc-by-4.0" || lic_lower == "cc-by-3.0" {
        let attr = attribution.map(|s| s.trim()).unwrap_or("");
        if attr.is_empty() {
            return Err(format!("{} needs an attribution", lic));
        }
        return Ok(());
    }

    if lic_lower.contains("openrail") {
        return Err(format!("{}: OpenRAIL licence, check its use restrictions", lic));
    }

    if lic_lower.split(['-', ' ']).any(|p| p == "nc") || lic_lower.contains("non-commercial") {
        return Err(format!("{}: non-commercial licence", lic));
    }

    Err(format!("{}: unknown licence, check it before use", lic))
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn test_allowed_simple() {
        assert!(check("Apache-2.0", None).is_ok());
        assert!(check(" mit ", None).is_ok());
        assert!(check("CC0-1.0", None).is_ok());
    }

    #[test]
    fn test_cc_by_requires_attribution() {
        assert!(check("CC-BY-4.0", Some("Artist X")).is_ok());
        assert_eq!(
            check("CC-BY-4.0", None).unwrap_err(),
            "CC-BY-4.0 needs an attribution"
        );
        assert_eq!(
            check("CC-BY-4.0", Some("  ")).unwrap_err(),
            "CC-BY-4.0 needs an attribution"
        );
    }

    #[test]
    fn test_warned_openrail_and_non_commercial() {
        assert_eq!(
            check("CreativeML OpenRAIL-M", None).unwrap_err(),
            "CreativeML OpenRAIL-M: OpenRAIL licence, check its use restrictions"
        );
        assert_eq!(
            check("CC-BY-NC-4.0", None).unwrap_err(),
            "CC-BY-NC-4.0: non-commercial licence"
        );
        assert_eq!(
            check("Stability non-commercial", None).unwrap_err(),
            "Stability non-commercial: non-commercial licence"
        );
    }

    #[test]
    fn test_unknown_licence() {
        assert_eq!(
            check("", None).unwrap_err(),
            "unknown: unknown licence, check it before use"
        );
        assert_eq!(
            check("Llama 3 Community", None).unwrap_err(),
            "Llama 3 Community: unknown licence, check it before use"
        );
    }
}
