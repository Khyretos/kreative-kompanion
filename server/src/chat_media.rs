//! CHAT-05: pictures and files that chat tools hand back.
use serde_json::Value;

/// The answer without markdown pictures the model made up: only urls in `keep` stay. A line left empty
/// by a removed picture goes away with the blank line after it.
pub fn strip_images(text: &str, keep: &[String]) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("![") {
        let after = &rest[at + 2..];
        let image = after.find("](").and_then(|b| {
            let url_start = b + 2;
            after[url_start..].find(')').map(|e| (url_start, url_start + e))
        });
        match image {
            Some((u0, u1)) if !after[..u0 - 2].contains('\n') => {
                out.push_str(&rest[..at]);
                if keep.iter().any(|k| k == &after[u0..u1]) {
                    out.push_str(&rest[at..at + 2 + u1 + 1]);
                }
                rest = &after[u1 + 1..];
            }
            _ => {
                out.push_str(&rest[..at + 2]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    let mut lines: Vec<&str> = Vec::new();
    let mut skip_blank = false;
    for (i, line) in out.split('\n').enumerate() {
        let was_image = text.split('\n').nth(i).is_some_and(|l| l.contains("!["));
        if skip_blank && line.is_empty() {
            skip_blank = false;
            continue;
        }
        skip_blank = false;
        if line.is_empty() && was_image {
            skip_blank = true;
            continue;
        }
        lines.push(line);
    }
    lines.join("\n")
}

/// The files a tools/call result embeds as resources: (mime type, base64 data, file name).
pub fn tool_files(result: &Value) -> Vec<(String, String, String)> {
    let Some(items) = result["content"].as_array() else { return Vec::new() };
    items
        .iter()
        .filter(|i| i["type"].as_str() == Some("resource"))
        .filter_map(|i| {
            let r = &i["resource"];
            let (mime, blob) = (r["mimeType"].as_str()?, r["blob"].as_str()?);
            let name = r["uri"].as_str().and_then(|u| u.rsplit('/').next()).filter(|n| !n.is_empty()).unwrap_or("file");
            Some((mime.to_string(), blob.to_string(), name.to_string()))
        })
        .collect()
}

#[cfg(test)]
#[path = "chat_media_tests.rs"]
mod tests;
