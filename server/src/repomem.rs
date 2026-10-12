//! KG-06: repo memory. A task's worker asks small questions about the project folder instead of
//! reading files to find its way: `repo_map` (where things are, plus the repo's conventions),
//! `repo_symbols` (where a name is defined) and `repo_decisions` (what the notes say about a word).
//! Each answer is capped, so a 9B model's context carries only the slice it asked for. The questions
//! run as shell steps on the computer in the task's folder (the same grant as any shell step), so
//! this module only builds the commands and trims their output.
use serde_json::{Value, json};

/// Longest answer given back to the model, in characters.
pub const MAX_ANSWER: usize = 3000;

/// The tool names (also the names the worker calls).
pub const TOOLS: [&str; 3] = ["repo_map", "repo_symbols", "repo_decisions"];

pub fn is_tool(name: &str) -> bool {
    TOOLS.contains(&name)
}

/// The function specs for the three tools.
pub fn schema() -> Vec<Value> {
    let spec = |name: &str, desc: &str, with_query: bool| {
        let props = if with_query { json!({ "query": { "type": "string" } }) } else { json!({}) };
        let required = if with_query { json!(["query"]) } else { json!([]) };
        json!({ "type": "function", "function": { "name": name, "description": desc,
            "parameters": { "type": "object", "properties": props, "required": required } } })
    };
    vec![
        spec("repo_map", "Where things are in the project: files per folder, and the repo's conventions. Ask this first; far cheaper than listing folders.", false),
        spec("repo_symbols", "Where a function, struct, class or type whose name contains `query` is defined (file:line and signature, at most 25).", true),
        spec("repo_decisions", "Lines of the project's decision and lesson notes that mention `query` (at most 15).", true),
    ]
}

/// Single-quotes a word for `sh`.
fn quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', "'\\''"))
}

/// A search word: one line, 1 to 80 characters.
fn query_of(args: &Value) -> Option<String> {
    let q = args["query"].as_str()?.trim();
    (!q.is_empty() && q.chars().count() <= 80 && !q.contains('\n')).then(|| q.to_string())
}

const DEFINITION: &str = r"^[[:space:]]*(pub(\([a-z]+\))? )?(export )?(default )?(async )?(unsafe )?(fn|struct|enum|trait|impl|type|const|static|mod|def|class|function|interface)[[:space:]]";

/// The shell command that answers a repo tool call in the task's folder, or None for a wrong call.
pub fn command(tool: &str, args: &Value) -> Option<String> {
    match tool {
        "repo_map" => Some(
            "git ls-files | sed -E 's#^([^/]+/[^/]+)/.*#\\1/#' | sort | uniq -c | sort -k2 | head -60; \
             for f in CLAUDE.md CONVENTIONS.md docs/conventions.md; do [ -f \"$f\" ] && { echo \"== $f\"; head -40 \"$f\"; }; done; true"
                .to_string(),
        ),
        "repo_symbols" => {
            let q = query_of(args)?;
            Some(format!("git grep -n -I -E {} | grep -i -F -- {} | head -25", quote(DEFINITION), quote(&q)))
        }
        "repo_decisions" => {
            let q = query_of(args)?;
            Some(format!(
                "grep -rniI -F -- {} docs/decisions* DECISIONS.md docs/lessons-learned.md 2>/dev/null | head -15; true",
                quote(&q)
            ))
        }
        _ => None,
    }
}

/// The step the worker's call becomes: a shell job in the folder.
pub fn job(tool: &str, args: &Value, folder: &str) -> Option<Value> {
    Some(json!({ "tool": "shell", "cwd": folder, "command": command(tool, args)? }))
}

/// The answer, cut to MAX_ANSWER characters at a line end, with a note when it was cut.
pub fn trim(answer: &str) -> String {
    if answer.chars().count() <= MAX_ANSWER {
        return if answer.trim().is_empty() { "(nothing found)".into() } else { answer.to_string() };
    }
    let head: String = answer.chars().take(MAX_ANSWER).collect();
    let cut = head.rfind('\n').unwrap_or(head.len());
    format!("{}\n... (cut: ask a narrower query)", &head[..cut])
}

#[cfg(test)]
#[path = "repomem_tests.rs"]
mod tests;
