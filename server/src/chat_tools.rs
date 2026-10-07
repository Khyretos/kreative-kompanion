//! CHAT-01: MCP tools in chats: a few tool rounds before the answer streams.

use std::{collections::HashMap, time::Duration};
use serde_json::{Value, json};
use crate::{AppState, config::{McpServerConfig, ProviderConfig}, llm};

const MAX_ROUNDS: usize = 4;
const CALL_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_RESULT: usize = 6000;

#[derive(Debug, Clone)]
pub struct ToolUse { pub server: String, pub tool: String, pub args: Value, pub result: String }

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
    u.args.as_object()?.values().find_map(|v| v.as_str()).map(|t| t.chars().take(60).collect())
}

pub fn use_line(u: &ToolUse) -> String {
    match what(u) {
        Some(w) => format!("*Looked up {}: {} ({w})*", u.server, u.tool),
        None => format!("*Looked up {}: {}*", u.server, u.tool),
    }
}

pub fn results_note(uses: &[ToolUse]) -> String {
    let parts: Vec<String> = uses
        .iter()
        .map(|u| format!("{}: {}{}\n{}", u.server, u.tool, what(u).map(|w| format!(" ({w})")).unwrap_or_default(), u.result))
        .collect();
    format!("You looked these up with tools for this answer. Use them, and say where an answer comes from.\n\n{}", parts.join("\n\n"))
}

/// CHAT-02: the built-in web tools' server name in a chat's tool list.
pub const WEB: &str = "Web";

async fn web_call(http: &reqwest::Client, w: &crate::config::SearchConfig, tool: &str, args: &Value) -> anyhow::Result<String> {
    match tool {
        "web_search" => crate::web::web_search(http, w.searxng_url.as_deref().unwrap_or(""), args["query"].as_str().unwrap_or("")).await,
        "read_page" => crate::web::read_page(args["url"].as_str().unwrap_or(""), w.allow_private).await,
        _ => anyhow::bail!("there is no tool {tool}"),
    }
}

pub async fn run(s: &AppState, p: &ProviderConfig, model: &str, convo: &[Value], servers: &[McpServerConfig], web: Option<&crate::config::SearchConfig>, on_use: &(dyn Fn(&ToolUse) + Send + Sync)) -> Vec<ToolUse> {
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
    
    if tools.is_empty() {
        return Vec::new();
    }
    
    let (specs, map) = tool_specs(&tools);
    let mut msgs = convo.to_vec();
    let mut uses = Vec::new();
    
    for _ in 0..MAX_ROUNDS {
        let Ok(ans) = llm::chat_with_tools(&s.http, p, model, &msgs, &specs).await else { break };
        
        let calls = ans["tool_calls"].as_array().cloned().unwrap_or_default();
        if calls.is_empty() { break; }
        
        msgs.push(ans.clone());
        
        for call in calls {
            let name = call["function"]["name"].as_str().unwrap_or("").to_string();
            let args_str = call["function"]["arguments"].as_str().unwrap_or("{}");
            let args: Value = serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));
            
            let result = match map.get(&name).cloned() {
                None => format!("error: there is no tool {name}"),
                Some((srv, tool)) if srv == WEB => match web {
                    Some(w) => match tokio::time::timeout(CALL_TIMEOUT, web_call(&s.http, w, &tool, &args)).await {
                        Ok(Ok(text)) => text,
                        Ok(Err(e)) => format!("error: {e}"),
                        Err(_) => "error: the tool took too long".to_string(),
                    },
                    None => "error: web search is not set up".to_string(),
                },
                Some((srv, tool)) => match servers.iter().find(|x| x.name == srv) {
                    None => format!("error: there is no tool {name}"),
                    Some(cfg) => match tokio::time::timeout(CALL_TIMEOUT, crate::mcp::call_tool(&s.http, cfg, &tool, &args)).await {
                        Ok(Ok(text)) => text,
                        Ok(Err(e)) => format!("error: {e}"),
                        Err(_) => "error: the tool took too long".to_string(),
                    },
                },
            };
            
            let result: String = result.chars().take(MAX_RESULT).collect();
            msgs.push(json!({"role": "tool", "tool_call_id": call["id"], "content": result}));
            
            if let Some((srv, tool)) = map.get(&name) {
                let u = ToolUse { server: srv.clone(), tool: tool.clone(), args, result };
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
