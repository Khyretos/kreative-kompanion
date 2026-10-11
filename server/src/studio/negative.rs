//! STU-N1: the negative prompt a person typed in the Studio form, for one workflow.
//! `default` is None when the workflow has no "negative" parameter, else that parameter's default.

pub fn negative_param(user: Option<&str>, label: &str, default: Option<&str>) -> Result<Option<String>, String> {
    let trim = |s: &str| s.trim_matches(|c: char| c == ',' || c.is_whitespace()).to_string();
    let cleaned = trim(user.unwrap_or(""));
    if cleaned.is_empty() {
        return Ok(None);
    }
    if cleaned.chars().count() > 500 {
        return Err("Keep the negative prompt under 500 characters.".to_string());
    }
    let Some(default) = default else {
        return Err(format!("{label} takes no negative prompt."));
    };
    // The adult-only guard puts "adult, mature" in every prompt; the negative may not undo it.
    let lower = cleaned.to_lowercase();
    if lower.split(|c: char| !c.is_alphanumeric()).any(|w| matches!(w, "adult" | "adults" | "mature")) {
        return Err("Adult and mature stay in every picture; take them out of the negative prompt.".to_string());
    }
    let d = trim(default);
    Ok(Some(if d.is_empty() { cleaned } else { format!("{d}, {cleaned}") }))
}

#[cfg(test)]
#[path = "negative_tests.rs"]
mod tests;
