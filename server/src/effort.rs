//! How hard the models work (EF-01): Auto, Low, Medium, High.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Effort {
    #[default]
    Auto,
    Low,
    Medium,
    High,
}

impl Effort {
    pub fn parse(s: &str) -> Option<Effort> {
        let trimmed = s.trim().to_lowercase();
        match trimmed.as_str() {
            "auto" => Some(Effort::Auto),
            "low" => Some(Effort::Low),
            "medium" => Some(Effort::Medium),
            "high" => Some(Effort::High),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Effort::Auto => "auto",
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Effort::Auto => "Auto",
            Effort::Low => "Low",
            Effort::Medium => "Medium",
            Effort::High => "High",
        }
    }

    pub fn resolve(self) -> Effort {
        if self == Effort::Auto {
            Effort::Medium
        } else {
            self
        }
    }

    pub fn tool_rounds(self) -> usize {
        match self {
            Effort::Low => 2,
            Effort::High => 10,
            Effort::Medium | Effort::Auto => 6,
        }
    }

    pub fn fix_rounds(self) -> usize {
        match self {
            Effort::Low => 1,
            Effort::Medium | Effort::High | Effort::Auto => 3,
        }
    }

    /// Pick an effort level from a list of skill efforts or heuristics.
    /// Returns High if any skill is High, Medium if any is Medium, Low if any is Low (ignoring Auto).
    /// If no skills are present, uses steps >= 5 or description_len > 2000 for High,
    /// steps <= 1 and description_len < 400 for Low, otherwise Medium.
    pub fn auto_pick(skill_efforts: &[Effort], steps: usize, description_len: usize) -> Effort {
        // Rule (a): check skill levels, ignoring Auto
        if skill_efforts.iter().any(|e| *e == Effort::High) {
            return Effort::High;
        }
        if skill_efforts.iter().any(|e| *e == Effort::Medium) {
            return Effort::Medium;
        }
        if skill_efforts.iter().any(|e| *e == Effort::Low) {
            return Effort::Low;
        }

        // Rule (b): heuristic fallback
        if steps >= 5 || description_len > 2000 {
            Effort::High
        } else if steps <= 1 && description_len < 400 {
            Effort::Low
        } else {
            Effort::Medium
        }
    }

    /// Resolve the effort a role should use for a task.
    /// Uses the task level if not Auto; otherwise uses the role's default (parsed) if Low/Medium/High,
    /// or falls back to `picked` if the role default is Auto or invalid.
    pub fn for_role(task: Effort, role_default: &str, picked: Effort) -> Effort {
        if task != Effort::Auto {
            return task;
        }
        if let Some(default) = Effort::parse(role_default) {
            if matches!(default, Effort::Low | Effort::Medium | Effort::High) {
                return default;
            }
        }
        picked
    }
}

/// The provider and model for one call at this effort: the level's extra_body merged over the
/// provider's (a level key replaces the provider's key), and the level's model when it sets one.
/// Auto resolves first; a level without a mapping leaves both unchanged.
pub fn apply(p: &crate::config::ProviderConfig, model: &str, e: Effort) -> (crate::config::ProviderConfig, String) {
    let mut out = p.clone();
    let Some(m) = p.effort.get(e.resolve().as_str()) else {
        return (out, model.to_string());
    };
    if let Some(extra) = &m.extra_body {
        let body = out.extra_body.get_or_insert_with(toml::Table::new);
        for (k, v) in extra {
            body.insert(k.clone(), v.clone());
        }
    }
    (out, m.model.clone().unwrap_or_else(|| model.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips() {
        assert_eq!(Effort::parse("auto"), Some(Effort::Auto));
        assert_eq!(Effort::parse("AUTO"), Some(Effort::Auto));
        assert_eq!(Effort::parse("  Auto  "), Some(Effort::Auto));
        assert_eq!(Effort::parse("low"), Some(Effort::Low));
        assert_eq!(Effort::parse("MEDIUM"), Some(Effort::Medium));
        assert_eq!(Effort::parse("HIGH"), Some(Effort::High));
        assert_eq!(Effort::parse("max"), None);
    }

    #[test]
    fn resolve_auto_becomes_medium() {
        assert_eq!(Effort::resolve(Effort::Auto), Effort::Medium);
        assert_eq!(Effort::resolve(Effort::Low), Effort::Low);
        assert_eq!(Effort::resolve(Effort::Medium), Effort::Medium);
        assert_eq!(Effort::resolve(Effort::High), Effort::High);
    }

    #[test]
    fn tool_rounds() {
        assert_eq!(Effort::tool_rounds(Effort::Low), 2);
        assert_eq!(Effort::tool_rounds(Effort::Medium), 6);
        assert_eq!(Effort::tool_rounds(Effort::High), 10);
        assert_eq!(Effort::tool_rounds(Effort::Auto), 6);
    }

    #[test]
    fn auto_pick_rules() {
        // Rule (a): skill levels take precedence
        assert_eq!(Effort::auto_pick(&[Effort::High], 0, 0), Effort::High);
        assert_eq!(Effort::auto_pick(&[Effort::Medium], 0, 0), Effort::Medium);
        assert_eq!(Effort::auto_pick(&[Effort::Low], 0, 0), Effort::Low);
        assert_eq!(Effort::auto_pick(&[Effort::Auto], 3, 100), Effort::Medium); // Auto ignored in skills
        assert_eq!(Effort::auto_pick(&[Effort::Auto, Effort::High], 0, 0), Effort::High);

        // Rule (b): heuristic fallback when no skills
        assert_eq!(Effort::auto_pick(&[], 5, 0), Effort::High); // steps >= 5
        assert_eq!(Effort::auto_pick(&[], 0, 2001), Effort::High); // description_len > 2000
        assert_eq!(Effort::auto_pick(&[], 0, 399), Effort::Low); // steps <= 1 && description_len < 400
        assert_eq!(Effort::auto_pick(&[], 1, 400), Effort::Medium); // otherwise
        assert_eq!(Effort::auto_pick(&[], 2, 300), Effort::Medium); // otherwise
        assert_eq!(Effort::auto_pick(&[Effort::Auto], 0, 0), Effort::Low); // no skill level, one small step
        assert_eq!(Effort::auto_pick(&[Effort::Low], 0, 0), Effort::Low);
    }

    #[test]
    fn for_role_order() {
        // Task level overrides everything if not Auto
        assert_eq!(Effort::for_role(Effort::High, "low", Effort::Low), Effort::High);
        assert_eq!(Effort::for_role(Effort::Medium, "high", Effort::High), Effort::Medium);

        // If task is Auto, use role default if valid (Low/Medium/High)
        assert_eq!(Effort::for_role(Effort::Auto, "low", Effort::High), Effort::Low);
        assert_eq!(Effort::for_role(Effort::Auto, "medium", Effort::High), Effort::Medium);
        assert_eq!(Effort::for_role(Effort::Auto, "high", Effort::High), Effort::High);

        // If role default is Auto or invalid, fall back to picked
        assert_eq!(Effort::for_role(Effort::Auto, "auto", Effort::High), Effort::High);
        assert_eq!(Effort::for_role(Effort::Auto, "invalid", Effort::Low), Effort::Low);
    }

    #[test]
    fn apply_merges_effort_config() {
        let input = r#"
id = "local"
name = "L"
kind = "openai-compatible"
base_url = "http://x/v3"
extra_body = { chat_template_kwargs = { enable_thinking = false }, top_k = 20 }
[effort.high]
extra_body = { chat_template_kwargs = { enable_thinking = true } }
[effort.low]
model = "Small"
"#;
        let p: crate::config::ProviderConfig = toml::from_str(input).unwrap();

        // High effort
        let (new_p, new_model) = apply(&p, "qwen3:14b", Effort::High);
        assert_eq!(new_model, "qwen3:14b");
        let tbl = new_p.extra_body.as_ref().unwrap();
        let kwargs = tbl
            .get("chat_template_kwargs")
            .unwrap()
            .as_table()
            .unwrap();
        assert_eq!(kwargs.get("enable_thinking").unwrap().as_bool(), Some(true));
        assert_eq!(tbl.get("top_k").unwrap().as_integer(), Some(20));

        // Low effort
        let (new_p, new_model) = apply(&p, "qwen3:14b", Effort::Low);
        assert_eq!(new_model, "Small");
        let tbl = new_p.extra_body.as_ref().unwrap();
        let kwargs = tbl
            .get("chat_template_kwargs")
            .unwrap()
            .as_table()
            .unwrap();
        assert_eq!(kwargs.get("enable_thinking").unwrap().as_bool(), Some(false));
        assert_eq!(tbl.get("top_k").unwrap().as_integer(), Some(20));

        // Medium effort (no mapping)
        let (new_p, new_model) = apply(&p, "qwen3:14b", Effort::Medium);
        assert_eq!(new_model, "qwen3:14b");
        assert_eq!(new_p.extra_body, p.extra_body);
    }
}
