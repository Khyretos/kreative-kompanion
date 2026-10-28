//! CHAT-09b: `summarize_attachment`, a digest of a big attached file for a question about the whole of it. The text is split
//! into parts that fit the model's window, one short model call digests each part, and the part digests are returned with
//! their line ranges (or merged first when they are too long to return). At most `MAX_PARTS` parts a use: the answer says
//! where to go on, so the model can carry on or read the details of one part with read_attachment. Each call costs a model
//! call per part, so a small question about a big file is better served by search_attachment.

use std::{io::BufRead, time::Duration};
use serde_json::{Value, json};
use crate::{AppState, api::RoleAssignment, attach_read::{self, Budget, Doc}, config::ProviderConfig, effort::Effort, llm};

pub const SUMMARIZE: &str = "summarize_attachment";
/// Why these calls are in the call log; like a chat summary they are not the chat's context use (the meter skips them).
pub const DIGEST_REASON: &str = "Digested a part of an attached file.";
/// Parts one use digests at most (about 2 minutes on a local 9B model).
pub const MAX_PARTS: usize = 8;
const CALL_TIMEOUT: Duration = Duration::from_secs(150);
const FOCUS_MAX: usize = 200;

const MAP_PROMPT: &str = "You write a faithful digest of one part of a file, for another model that cannot read it. Keep names, numbers, dates, \
headings, decisions, commands and error messages exactly as written. No opinions and no introduction. Reply with at most 8 \
bullet points, one line each, each starting with '- '.";
const MERGE_PROMPT: &str = "These are digests of consecutive parts of one file, each marked with its lines. Merge them into one digest of the \
whole, in order, keeping the line marks and every name, number and date. At most 12 bullet points, one line each, each starting with '- '.";

/// Who a digest is made for and with: the answering model (at low effort, so a thinking model does not think over every part).
pub struct Digester {
    pub p: ProviderConfig,
    pub model: String,
    pub user_id: String,
    pub role: RoleAssignment,
}

impl Digester {
    pub fn new(p: &ProviderConfig, role: &RoleAssignment, user_id: &str) -> Self {
        let (p, model) = crate::effort::apply(p, &role.model_id, Effort::Low);
        Digester { p, model, user_id: user_id.to_string(), role: role.clone() }
    }
}

/// One part of the file: its first and last line and its text.
type Part = (usize, usize, String);

/// The parts from line `from` on, in one pass: each up to `size` characters, at most `MAX_PARTS`. The flag says the file goes on.
pub fn split_parts<R: BufRead>(r: R, from: usize, size: usize) -> std::io::Result<(Vec<Part>, bool)> {
    let (mut parts, mut more) = (Vec::<Part>::new(), false);
    let (mut text, mut first, mut last) = (String::new(), 0usize, 0usize);
    attach_read::scan(r, |n, line| {
        if n < from { return true }
        if !text.is_empty() && text.chars().count() + line.chars().count() + 1 > size {
            parts.push((first, last, std::mem::take(&mut text)));
            if parts.len() >= MAX_PARTS {
                more = true;
                return false;
            }
        }
        if text.is_empty() { first = n }
        text.push_str(line);
        text.push('\n');
        last = n;
        true
    })?;
    if !text.is_empty() { parts.push((first, last, text)) }
    Ok((parts, more))
}

fn marked(parts: &[Part]) -> String {
    parts.iter().map(|(a, b, t)| format!("Lines {a}-{b}:\n{}", t.trim_end())).collect::<Vec<_>>().join("\n\n")
}

/// One model call: the digest of `user`, logged like a chat summary. A refusal that names the model's limit is learned.
async fn ask(s: &AppState, d: &Digester, chat_id: &str, system: &str, user: String) -> Result<String, String> {
    let messages = [json!({ "role": "system", "content": system }), json!({ "role": "user", "content": &user })];
    let started = std::time::Instant::now();
    let answer = tokio::time::timeout(CALL_TIMEOUT, llm::chat_with_tools_full(&s.http, &d.p, &d.model, &messages, &json!([]))).await;
    let (msg, usage, mut error) = match answer {
        Ok(Ok((m, u))) => (m, u, None),
        Ok(Err(e)) => (Value::Null, Value::Null, Some(format!("{e:#}"))),
        Err(_) => (Value::Null, Value::Null, Some("the model took too long".to_string())),
    };
    let text = msg["content"].as_str().unwrap_or("").trim().to_string();
    if error.is_none() && text.is_empty() { error = Some("the model wrote an empty digest".into()) }
    let note = match &error {
        Some(e) => crate::context_windows::learn(s, &d.p.id, &d.model, e, system.chars().count() + user.chars().count()).await,
        None => None,
    };
    if let (Some(e), Some(note)) = (error.clone(), note) { error = Some(format!("{e}. {note}")) }
    let request = json!({ "system": system, "user": crate::util::cut_marked(&user, 2_000), "user_chars": user.chars().count() });
    crate::api::log_call(s, &d.user_id, chat_id, None, &d.role.role, &d.role, DIGEST_REASON, &request, &json!(text), &usage,
        started.elapsed().as_millis(), error.as_deref(), Effort::Low).await;
    error.map_or(Ok(text), Err)
}

/// Merges digests in groups that fit one call, over and over, until they fit `room` (or cannot be merged further).
async fn merge(s: &AppState, d: &Digester, chat_id: &str, mut layer: Vec<Part>, size: usize, room: usize) -> Result<String, String> {
    loop {
        let joined = marked(&layer);
        if joined.chars().count() <= room || layer.len() == 1 { return Ok(crate::util::cut_marked(&joined, room)) }
        let mut groups: Vec<Vec<Part>> = Vec::new();
        for part in layer.iter().cloned() {
            match groups.last_mut() {
                Some(g) if marked(g).chars().count() + part.2.chars().count() + 30 <= size => g.push(part),
                _ => groups.push(vec![part]),
            }
        }
        if groups.len() == layer.len() { return Ok(crate::util::cut_marked(&joined, room)) }
        let mut next = Vec::new();
        for g in groups {
            if g.len() == 1 {
                next.extend(g);
                continue;
            }
            let (a, b) = (g[0].0, g[g.len() - 1].1);
            next.push((a, b, ask(s, d, chat_id, MERGE_PROMPT, marked(&g)).await?));
        }
        layer = next;
    }
}

/// The tool: a digest of the file from `from_line` (default the start), at most `MAX_PARTS` parts, within `room` characters.
pub async fn run(s: &AppState, d: &Digester, chat_id: &str, docs: &[Doc], args: &Value, room: usize, b: Budget) -> Result<String, String> {
    let name = args["name"].as_str().map(str::trim).filter(|n| !n.is_empty());
    let doc = attach_read::pick(docs, name)?;
    let path = doc.text.clone().ok_or("this file has no text to digest")?;
    let from = args["from_line"].as_u64().map_or(1, |n| n.max(1) as usize);
    let focus = args["focus"].as_str().map(|f| f.trim().chars().take(FOCUS_MAX).collect::<String>()).filter(|f| !f.is_empty());
    let size = b.part;
    let read = tokio::task::spawn_blocking(move || split_parts(std::io::BufReader::new(std::fs::File::open(path)?), from, size)).await;
    let (parts, more) = read.map_err(|e| format!("error: {e}"))?.map_err(|e| format!("error: {e}"))?;
    if parts.is_empty() { return Ok(format!("{} has nothing from line {from} on.", doc.name)) }
    let ask_focus = focus.as_ref().map(|f| format!("\nThe reader cares most about: {f}\n")).unwrap_or_default();
    let (mut digests, mut stopped) = (Vec::<Part>::new(), None::<String>);
    for (a, z, text) in &parts {
        let user = format!("File: {}, lines {a}-{z}{ask_focus}\n\n{text}", doc.name);
        match ask(s, d, chat_id, MAP_PROMPT, user).await {
            Ok(t) => digests.push((*a, *z, t)),
            Err(e) => {
                stopped = Some(format!("lines {a}-{z}: {e}"));
                break;
            }
        }
    }
    let Some(&(first, _, _)) = digests.first() else { return Err(format!("error: no part could be digested ({})", stopped.unwrap_or_default())) };
    let last = digests.last().map_or(first, |p| p.1);
    let n = digests.len();
    let body = match merge(s, d, chat_id, digests.clone(), size, room.saturating_sub(500).max(600)).await {
        Ok(b) => b,
        Err(_) => crate::util::cut_marked(&marked(&digests), room.saturating_sub(500).max(600)),
    };
    let mut out = format!("Digest of {}, lines {first}-{last}{}:\n{body}\n\n", doc.name, focus.map(|f| format!(" (focus: {f})")).unwrap_or_default());
    out.push_str(&format!("[{n} part{} digested", if n == 1 { "" } else { "s" }));
    if let Some(e) = stopped {
        out.push_str(&format!("; stopped at {e}"));
    }
    if more || n < parts.len() {
        out.push_str(&format!(". To digest on: {SUMMARIZE} name=\"{}\" from_line={}", doc.name, last + 1));
    } else {
        out.push_str(". This is the end of the file");
    }
    out.push_str(&format!(". For the details of a part: {} name=\"{}\" from_line=N.]", attach_read::READ, doc.name));
    Ok(out)
}

#[cfg(test)]
#[path = "attach_digest_tests.rs"]
mod tests;
