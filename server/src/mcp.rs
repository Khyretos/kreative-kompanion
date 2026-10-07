//! CHAT-01: MCP tool servers over streamable HTTP: list their tools (cached) and call one.

use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use crate::config::McpServerConfig;

const TIMEOUT: Duration = Duration::from_secs(30);
const CACHE_FOR: Duration = Duration::from_secs(300);
static TOOLS: Mutex<Option<HashMap<String, (Instant, Vec<Value>)>>> = Mutex::new(None);
/// A server that did not answer is asked again only after this (the Capabilities page polls).
const RETRY_AFTER: Duration = Duration::from_secs(60);
static DOWN: Mutex<Option<HashMap<String, (Instant, String)>>> = Mutex::new(None);

/// The JSON-RPC answer's "result" from a JSON body or the last `data:` event with an id.
pub fn parse_response(content_type: &str, body: &str) -> Result<Value> {
    let v: Value = if content_type.contains("text/event-stream") {
        body.lines()
            .rev()
            .filter_map(|l| l.strip_prefix("data:"))
            .filter_map(|d| serde_json::from_str::<Value>(d.trim()).ok())
            .find(|v| v.get("id").is_some())
            .context("no JSON-RPC answer")?
    } else {
        serde_json::from_str(body).context("no JSON-RPC answer")?
    };
    if let Some(err) = v.get("error") {
        bail!("{}", err["message"].as_str().map(String::from).unwrap_or_else(|| err.to_string()));
    }
    Ok(v.get("result").cloned().unwrap_or(Value::Null))
}

/// The text of a tools/call result (text items only).
pub fn tool_text(result: &Value) -> String {
    result["content"]
        .as_array()
        .map(|items| {
            items.iter()
                .filter(|i| i["type"] == "text")
                .filter_map(|i| i["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

async fn rpc(http: &reqwest::Client, s: &McpServerConfig, session: Option<&str>, method: &str, params: Value, id: Option<u64>) -> Result<(Value, Option<String>)> {
    let mut body = json!({"jsonrpc": "2.0", "method": method, "params": params});
    if let Some(id) = id {
        body["id"] = json!(id);
    }
    let mut req = http.post(&s.url)
        .header("Accept", "application/json, text/event-stream")
        .timeout(TIMEOUT)
        .json(&body);
    if let Some(token) = s.token_env.as_deref().and_then(|e| std::env::var(e).ok()).filter(|t| !t.is_empty()) {
        req = req.bearer_auth(token);
    }
    if let Some(sid) = session {
        req = req.header("mcp-session-id", sid);
    }
    let resp = req.send().await.with_context(|| format!("{} did not answer", s.name))?;
    let status = resp.status();
    if !status.is_success() {
        bail!("{} answered {}", s.name, status);
    }
    let sid = resp.headers().get("mcp-session-id").and_then(|v| v.to_str().ok()).map(String::from);
    if id.is_none() {
        return Ok((Value::Null, sid));
    }
    let ctype = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let text = resp.text().await?;
    Ok((parse_response(&ctype, &text)?, sid))
}

/// initialize + notifications/initialized; the server's session id, if it uses one.
async fn session(http: &reqwest::Client, s: &McpServerConfig) -> Result<Option<String>> {
    let params = json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "kompanion", "version": env!("CARGO_PKG_VERSION")}});
    let (_, sid) = rpc(http, s, None, "initialize", params, Some(1)).await?;
    let _ = rpc(http, s, sid.as_deref(), "notifications/initialized", json!({}), None).await;
    Ok(sid)
}

pub async fn list_tools(http: &reqwest::Client, s: &McpServerConfig) -> Result<Vec<Value>> {
    let sid = session(http, s).await?;
    let (res, _) = rpc(http, s, sid.as_deref(), "tools/list", json!({}), Some(2)).await?;
    
    let tools = res.get("tools")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    
    Ok(tools)
}

pub async fn call_tool(http: &reqwest::Client, s: &McpServerConfig, name: &str, args: &Value) -> Result<String> {
    let sid = session(http, s).await?;
    let (res, _) = rpc(http, s, sid.as_deref(), "tools/call", json!({
        "name": name,
        "arguments": args
    }), Some(3)).await?;

    if res.get("isError").and_then(|v| v.as_bool()) == Some(true) {
        bail!("{}", tool_text(&res));
    }

    Ok(tool_text(&res))
}

/// list_tools, remembered per server for CACHE_FOR.
pub async fn tools_cached(http: &reqwest::Client, s: &McpServerConfig) -> Result<Vec<Value>> {
    if let Some((at, tools)) = TOOLS.lock().unwrap().get_or_insert_with(HashMap::new).get(&s.name)
        && at.elapsed() < CACHE_FOR
    {
        return Ok(tools.clone());
    }
    let tools = list_tools(http, s).await?;
    TOOLS.lock().unwrap().get_or_insert_with(HashMap::new).insert(s.name.clone(), (Instant::now(), tools.clone()));
    Ok(tools)
}

/// Each configured server for the Capabilities page: off, ok with its tools, or down with the error.
pub async fn status(http: &reqwest::Client, servers: &[McpServerConfig]) -> Vec<Value> {
    let mut out = Vec::new();
    for s in servers {
        out.push(if !s.enabled {
            json!({"name": s.name, "status": "off", "description": s.description, "tools": []})
        } else {
            let down = DOWN.lock().unwrap().get_or_insert_with(HashMap::new).get(&s.name).filter(|(at, _)| at.elapsed() < RETRY_AFTER).map(|(_, e)| e.clone());
            let down_was = down.clone();
            let tools = match down {
                Some(e) => Err(anyhow::anyhow!(e)),
                None => tools_cached(http, s).await,
            };
            if let (Err(e), None) = (&tools, &down_was) {
                DOWN.lock().unwrap().get_or_insert_with(HashMap::new).insert(s.name.clone(), (Instant::now(), e.to_string()));
            } else if tools.is_ok() {
                DOWN.lock().unwrap().get_or_insert_with(HashMap::new).remove(&s.name);
            }
            match tools {
                Ok(t) => json!({"name": s.name, "status": "ok", "description": s.description,
                    "tools": t.iter().map(|x| json!({"name": x["name"], "description": x["description"]})).collect::<Vec<Value>>()}),
                Err(e) => json!({"name": s.name, "status": "down", "description": s.description, "error": e.to_string(), "tools": []}),
            }
        });
    }
    out
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
