//! CHAT-01: MCP tools in chats: a few tool rounds before the answer streams.

use std::{collections::HashMap, time::Duration};
use serde_json::{Value, json};
use crate::{AppState, config::{McpServerConfig, ProviderConfig}, llm};

const MAX_ROUNDS: usize = 4;
const CALL_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_RESULT: usize = 6000;

#[derive(Debug, Clone)]
pub struct ToolUse { pub server: String, pub tool: String, pub args: Value, pub result: String, pub images: Vec<String>, pub files: Vec<(String, String)>, pub sources: Vec<Source> }

/// CHAT-04: one thing an answer can cite: a web result or a knowledge part, numbered across the whole answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Source { pub n: usize, pub name: String, pub url: Option<String>, pub excerpt: String }

fn excerpt(t: &str) -> String {
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > 500 { format!("{}…", t.chars().take(500).collect::<String>()) } else { t }
}

/// The sources the answer cites (web: its link is in the text; knowledge: `src:n`), or all consulted when none match.
/// Appended to the saved answer as a `:::sources` line of JSON that the app shows as an expandable list.
pub fn sources_block(uses: &[ToolUse], answer: &str) -> String {
    let all: Vec<&Source> = uses.iter().flat_map(|u| u.sources.iter()).collect();
    let cited: Vec<&Source> = all.iter().copied().filter(|x| match &x.url {
        Some(u) => answer.contains(u.as_str()),
        None => answer.contains(&format!("src:{}", x.n)),
    }).collect();
    let shown = if cited.is_empty() { all } else { cited };
    if shown.is_empty() { return String::new(); }
    let list: Vec<Value> = shown.iter().map(|x| json!({"n": x.n, "name": x.name, "url": x.url, "excerpt": x.excerpt})).collect();
    format!("\n\n:::sources\n{}\n:::", Value::Array(list))
}

/// CHAT-04: the date, the server's name and place go into every chat prompt, so "how old is ..." is worked out from today.
pub fn when_where(now: time::OffsetDateTime, machine: Option<&str>, location: Option<&str>, web: bool) -> String {
    let (d, wd) = (now.date(), now.weekday());
    let mut t = format!("Today is {wd} {d}, {:02}:{:02} UTC.", now.hour(), now.minute());
    match (machine, location) {
        (Some(m), Some(l)) => t.push_str(&format!(" Kompanion is installed on {m}, in {l}.")),
        (Some(m), None) => t.push_str(&format!(" Kompanion is installed on {m}.")),
        (None, Some(l)) => t.push_str(&format!(" Kompanion is installed in {l}.")),
        _ => {}
    }
    t.push_str(" Your own knowledge ends before today: work out ages and \"how long ago\" from today's date, and treat office holders, news, prices and versions as possibly changed.");
    if web { t.push_str(" Web search is on: search before answering anything that can change, and never guess it."); }
    t
}

fn clean(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
}

pub fn fn_name(server: &str, tool: &str) -> String {
    let mut name = format!("{}__{}", clean(server), clean(tool));
    name.truncate(64);
    name
}

pub fn tool_specs(tools: &[(String, Vec<Value>)]) -> (Value, HashMap<String, (String, String)>) {
    let mut specs = Vec::new();
    let mut map = HashMap::new();
    
    for (srv_name, srv_tools) in tools {
        for t in srv_tools {
            let t_name = t["name"].as_str().unwrap_or("").to_string();
            if t_name.is_empty() { continue; }
            
            let name = fn_name(srv_name, &t_name);
            
            let desc = t["description"]
                .as_str()
                .map(|d| d.chars().take(300).collect::<String>())
                .unwrap_or_default();
            
            let params = if t["inputSchema"].is_object() && !t["inputSchema"].is_null() {
                t["inputSchema"].clone()
            } else {
                json!({"type": "object", "properties": {}})
            };
            
            let spec = json!({
                "type": "function",
                "function": {
                    "name": name.clone(),
                    "description": desc,
                    "parameters": params
                }
            });
            
            specs.push(spec);
            map.insert(name, (srv_name.clone(), t_name));
        }
    }
    
    (json!(specs), map)
}

/// The first string argument of a use, shortened: what it looked up.
fn what(u: &ToolUse) -> Option<String> {
    // One line, also for code arguments (BLD-01: Blender scripts).
    u.args.as_object()?.values().find_map(|v| v.as_str()).map(|t| t.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(60).collect())
}

pub fn use_line(u: &ToolUse) -> String {
    let line = match what(u) {
        Some(w) => format!("*Looked up {}: {} ({w})*", u.server, u.tool),
        None => format!("*Looked up {}: {}*", u.server, u.tool),
    };
    let pics: String = u.images.iter().map(|url| format!("\n\n![{}: {}]({url})", u.server, u.tool)).collect();
    // CHAT-05: files the tool made (a .blend scene) become download links under the picture.
    let files: String = u.files.iter().map(|(url, name)| format!("\n\n[{name}]({url})")).collect();
    format!("{line}{pics}{files}")
}

pub fn results_note(uses: &[ToolUse]) -> String {
    let parts: Vec<String> = uses
        .iter()
        .map(|u| format!("{}: {}{}\n{}", u.server, u.tool, what(u).map(|w| format!(" ({w})")).unwrap_or_default(), u.result))
        .collect();
    // BLD-01: a tool made a picture (a Blender render): the model must not deny it or offer it again.
    let pics = if uses.iter().any(|u| !u.images.is_empty() || !u.files.is_empty()) {
        "Your tools made the picture and files shown to the user above your answer, with Download buttons. Say in a sentence or two what the scene shows. Never write markdown images or links to pictures or files yourself, never say you cannot render or that tools are missing.\n\n"
    } else { "" };
    format!("{pics}You looked these up with tools for this answer. Use them, and always cite where each point comes from, right after the point: a web result as a markdown link with the site's name and its URL, like [Reuters](https://...); a knowledge part as [collection / document](src:n) with its number n. Never cite what you did not get here, and do not add a list of sources: the app shows them under your answer.\n\n{}", parts.join("\n\n"))
}

/// CHAT-02: the built-in web tools' server name in a chat's tool list.
pub const WEB: &str = "Web";
/// CHAT-03: the built-in knowledge search's name in a chat's tool list.
pub const KNOWLEDGE: &str = "Knowledge";

/// CHAT-03b: the query's embedding for the knowledge search; None when the Embedder is not there.
async fn knowledge_vector(s: &AppState, q: &str) -> Option<Vec<f32>> {
    let ai = crate::assets::ai::Ai::from_env(s.http.clone());
    tokio::time::timeout(Duration::from_secs(10), crate::assets::ai::embed_query(&ai, q)).await.ok().flatten()
}

async fn web_call(http: &reqwest::Client, w: &crate::config::SearchConfig, tool: &str, args: &Value, first_n: usize) -> anyhow::Result<(String, Vec<Source>)> {
    match tool {
        "web_search" => {
            let (text, found) = crate::web::web_search_from(http, w.searxng_url.as_deref().unwrap_or(""), args["query"].as_str().unwrap_or(""), first_n).await?;
            let sources = found.into_iter().enumerate().map(|(i, (name, url, c))| Source { n: first_n + i, name, url: Some(url), excerpt: excerpt(&c) }).collect();
            Ok((text, sources))
        }
        "read_page" => {
            let url = args["url"].as_str().unwrap_or("");
            let text = crate::web::read_page(url, w.allow_private).await?;
            let src = Source { n: first_n, name: url.to_string(), url: Some(url.to_string()), excerpt: excerpt(&text) };
            Ok((format!("[page {first_n}] {url}\n{text}"), vec![src]))
        }
        _ => anyhow::bail!("there is no tool {tool}"),
    }
}

pub async fn run(
    s: &AppState,
    p: &ProviderConfig,
    model: &str,
    convo: &[Value],
    servers: &[McpServerConfig],
    web: Option<&crate::config::SearchConfig>,
    knowledge: Option<(&str, Option<&str>)>,
    files: Option<(&std::path::Path, &str)>,
    on_use: &(dyn Fn(&ToolUse) + Send + Sync),
) -> Vec<ToolUse> {
    let mut tools = Vec::new();
    for srv in servers {
        if !srv.enabled { continue; }
        if let Ok(t) = crate::mcp::tools_cached(&s.http, srv).await {
            tools.push((srv.name.clone(), t));
        }
    }
    
    if let Some(w) = web.filter(|w| w.searxng_url.is_some()) {
        let _ = w;
        tools.push((WEB.to_string(), crate::web::tools()));
    }
    
    if let Some((uid, _)) = knowledge {
        let names: Vec<String> = crate::knowledge::collections(&s.db, uid).await.unwrap_or_default().into_iter().map(|c| c.0).collect();
        if !names.is_empty() {
            tools.push((KNOWLEDGE.to_string(), crate::knowledge::tools(&names)));
        }
    }
    
    if tools.is_empty() {
        return Vec::new();
    }
    
    let (specs, map) = tool_specs(&tools);
    let mut msgs = convo.to_vec();
    let mut uses = Vec::new();
    let mut next_n = 1usize;
    
    for _ in 0..MAX_ROUNDS {
        let Ok(ans) = llm::chat_with_tools(&s.http, p, model, &msgs, &specs).await else { break };
        
        let calls = ans["tool_calls"].as_array().cloned().unwrap_or_default();
        if calls.is_empty() { break; }
        
        msgs.push(ans.clone());
        
        for call in calls {
            let name = call["function"]["name"].as_str().unwrap_or("").to_string();
            let args_str = call["function"]["arguments"].as_str().unwrap_or("{}");
            let args: Value = serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));
            let mut images: Vec<String> = Vec::new();
            let mut files_made: Vec<(String, String)> = Vec::new();
            let mut sources: Vec<Source> = Vec::new();
            
            let result = match map.get(&name).cloned() {
                None => format!("error: there is no tool {name}"),
                Some((srv, tool)) if srv == WEB => match web {
                    Some(w) => match tokio::time::timeout(CALL_TIMEOUT, web_call(&s.http, w, &tool, &args, next_n)).await {
                        Ok(Ok((text, found))) => { sources = found; text },
                        Ok(Err(e)) => format!("error: {e}"),
                        Err(_) => "error: the tool took too long".to_string(),
                    },
                    None => "error: web search is not set up".to_string(),
                },
                Some((srv, _)) if srv == KNOWLEDGE => match knowledge {
                    Some((uid, project)) => match crate::knowledge::search(&s.db, uid, args["query"].as_str().unwrap_or(""), knowledge_vector(s, args["query"].as_str().unwrap_or("")).await.as_deref(), args["collection"].as_str().filter(|c| !c.is_empty()), project, 8).await {
                        Ok(hits) => {
                            sources = hits.iter().enumerate().map(|(i, h)| Source { n: next_n + i, name: format!("{} / {}", h.collection, h.doc), url: None, excerpt: excerpt(&h.text) }).collect();
                            crate::knowledge::hits_numbered(&hits, next_n)
                        }
                        Err(e) => format!("error: {e}"),
                    },
                    None => "error: knowledge is not turned on".to_string(),
                },
                Some((srv, tool)) => match servers.iter().find(|x| x.name == srv) {
                    None => format!("error: there is no tool {name}"),
                    Some(cfg) => match tokio::time::timeout(CALL_TIMEOUT, crate::mcp::call_tool_full(&s.http, cfg, &tool, &args)).await {
                        Ok(Ok((text, pics, made))) => {
                            if let Some((dir, chat)) = files {
                                for (mime, data) in pics {
                                    match crate::chat_files::save(dir, chat, &mime, &data).await {
                                        Ok(file) => images.push(format!("/api/chats/{chat}/files/{file}")),
                                        Err(e) => tracing::warn!("chat picture not saved: {e}"),
                                    }
                                }
                                for (mime, data, name) in made {
                                    match crate::chat_files::save(dir, chat, &mime, &data).await {
                                        Ok(file) => files_made.push((format!("/api/chats/{chat}/files/{file}"), name)),
                                        Err(e) => tracing::warn!("chat file not saved: {e}"),
                                    }
                                }
                            }
                            if images.is_empty() { text } else { format!("{text}\n(Done: the picture is rendered and the user already sees it above your answer{}. Say in a sentence or two what it shows; do not offer to render it again and do not write any image markdown.)", if files_made.is_empty() { "" } else { ", with a download for the scene file" }) }
                        }
                        Ok(Err(e)) => format!("error: {e}"),
                        Err(_) => "error: the tool took too long".to_string(),
                    },
                },
            };
            
            let result: String = result.chars().take(MAX_RESULT).collect();
            next_n += sources.len().max(1);
            msgs.push(json!({"role": "tool", "tool_call_id": call["id"], "content": result}));
            
            if let Some((srv, tool)) = map.get(&name) {
                let u = ToolUse { server: srv.clone(), tool: tool.clone(), args, result, images, files: files_made, sources };
                on_use(&u);
                uses.push(u);
            }
        }
    }
    
    uses
}

#[cfg(test)]
#[path = "chat_tools_tests.rs"]
mod tests;
