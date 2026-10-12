//! KG-04: the escalation ladder. A run that its worker cannot finish is retried by the next
//! rung (`[[ladder]]` in kompanion.toml, e.g. Coder, then a strong model on any provider) before
//! the user is asked. This holds the pure parts: which rung comes next, what a rung tried,
//! and the text the user gets at the end.
use serde::{Deserialize, Serialize};

use crate::config::LadderRung;

/// One rung of one run, as logged in `runs.ladder`: why it started, what it cost, how it ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tried {
    /// 0 is the worker role as assigned; 1.. are the `[[ladder]]` rungs used.
    pub rung: usize,
    pub name: String,
    pub provider: String,
    pub model: String,
    /// Why this rung ran: "worker role" or "after <why the one below failed>".
    pub reason: String,
    pub tokens: i64,
    pub minutes: i64,
    /// Fix rounds the rung had.
    pub rounds: i64,
    /// "done", or why it failed.
    pub outcome: String,
}

/// The next rung to try: the first in order whose provider is configured and whose provider and
/// model were not tried yet (`used` lists provider/model pairs, the worker role's first).
pub fn next_rung<'a>(
    rungs: &'a [LadderRung],
    used: &[(String, String)],
    configured: impl Fn(&str) -> bool,
) -> Option<&'a LadderRung> {
    rungs
        .iter()
        .find(|r| configured(&r.provider) && !used.iter().any(|(p, m)| *p == r.provider && *m == r.model))
}

/// What the user reads when every rung failed: one line per rung, what it was, what it cost, why it stopped.
pub fn ask_user_text(tried: &[Tried]) -> String {
    let mut out = format!("{} rung(s) tried, none finished the task:\n", tried.len());
    for t in tried {
        out.push_str(&format!(
            "- {} ({}): {} ({} tokens, {} min, {} fix round(s))\n",
            t.name, t.model, t.outcome, t.tokens, t.minutes, t.rounds
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rung(provider: &str, model: &str) -> LadderRung {
        LadderRung { name: None, provider: provider.into(), model: model.into(), fix_rounds: None }
    }

    #[test]
    fn the_next_rung_skips_models_already_tried_and_providers_that_are_not_configured() {
        let rungs = vec![rung("ovms", "Coder"), rung("gone", "x"), rung("anthropic", "claude-sonnet-5-5")];
        let all = |_: &str| true;
        let no_gone = |p: &str| p != "gone";
        let worker = vec![("ovms".to_string(), "Qwen9B".to_string())];
        assert_eq!(next_rung(&rungs, &worker, all).unwrap().model, "Coder");
        let mut used = worker.clone();
        used.push(("ovms".into(), "Coder".into()));
        assert_eq!(next_rung(&rungs, &used, no_gone).unwrap().model, "claude-sonnet-5-5");
        used.push(("anthropic".into(), "claude-sonnet-5-5".into()));
        assert!(next_rung(&rungs, &used, no_gone).is_none());
        // The worker role already is the first rung's model: it is not retried on itself.
        let same = vec![("ovms".to_string(), "Coder".to_string())];
        assert_eq!(next_rung(&rungs, &same, all).unwrap().model, "x");
        assert!(next_rung(&[], &worker, all).is_none());
    }

    #[test]
    fn the_user_text_says_what_each_rung_tried() {
        let t = |rung, name: &str, model: &str, outcome: &str| Tried {
            rung, name: name.into(), provider: "p".into(), model: model.into(), reason: "worker role".into(),
            tokens: 1200, minutes: 3, rounds: 2, outcome: outcome.into(),
        };
        let text = ask_user_text(&[t(0, "Worker", "Qwen9B", "review failed 3 times"), t(1, "Coder", "Coder", "step 2 needs you")]);
        assert!(text.starts_with("2 rung(s) tried"));
        assert!(text.contains("- Worker (Qwen9B): review failed 3 times (1200 tokens, 3 min, 2 fix round(s))"));
        assert!(text.contains("- Coder (Coder): step 2 needs you"));
    }

    #[test]
    fn ladder_rungs_load_from_toml() {
        #[derive(Deserialize)]
        struct Doc {
            ladder: Vec<LadderRung>,
        }
        let d: Doc = toml::from_str(
            "[[ladder]]\nprovider = \"ovms\"\nmodel = \"Coder\"\n[[ladder]]\nname = \"Claude\"\nprovider = \"anthropic\"\nmodel = \"claude-sonnet-5-5\"\nfix_rounds = 2\n",
        )
        .unwrap();
        assert_eq!(d.ladder[0].label(), "Coder");
        assert_eq!(d.ladder[1].label(), "Claude");
        assert_eq!(d.ladder[1].fix_rounds, Some(2));
    }
}
