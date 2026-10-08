use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attached {
    pub name: String,
    pub file: String,
    pub kind: String,
}

pub fn block(files: &[Attached]) -> String {
    if files.is_empty() {
        return String::new();
    }
    let json = serde_json::to_string(files).unwrap_or_default();
    format!("\n\n:::files\n{}", json)
}

pub fn split(text: &str) -> (String, Vec<Attached>) {
    if let Some(idx) = text.rfind("\n\n:::files\n") {
        let prefix = &text[..idx];
        let rest = &text[idx + "\n\n:::files\n".len()..];
        let suffix = rest.lines().next().unwrap_or("");
        if let Ok(files) = serde_json::from_str::<Vec<Attached>>(suffix) {
            return (prefix.to_string(), files);
        }
    }
    (text.to_string(), vec![])
}

pub fn kind_for(name: &str) -> Option<&'static str> {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase());
    match ext.as_deref()? {
        "png" | "jpg" | "jpeg" | "webp" => Some("image"),
        "pdf" | "md" | "markdown" | "txt" | "text" | "rst" | "csv" | "json" | "toml" | "yaml" | "yml" | "xml" | "html" | "htm" | "log" | "ini" | "cfg" | "sql" | "sh" | "css" | "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "gd" | "c" | "h" | "cpp" | "hpp" | "java" | "go" | "rb" | "kt" | "cs" => Some("text"),
        _ => None,
    }
}

pub fn sees_pictures(model: &str) -> bool {
    let lower = model.to_lowercase();
    // Explicit exclusions for substrings that might match 'vl' but are not vision models
    if ["coder", "autocomplete", "deepseek-chat", "llama3.1:8b"].iter().any(|e| lower.contains(e)) {
        return false;
    }
    ["vl", "vision", "llava", "gemma3", "claude", "gpt-4o", "pixtral", "minicpm-v", "qwen3.5", "gemini"]
        .iter()
        .any(|k| lower.contains(k))
}

#[cfg(test)]
#[path = "chat_attach_tests.rs"]
mod tests;
