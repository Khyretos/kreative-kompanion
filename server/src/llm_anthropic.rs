//! Tool calls on Anthropic's own Messages API. The rest of Kompanion speaks the OpenAI message
//! shape (system/user/assistant/tool, `tool_calls` with JSON-string arguments); this module turns
//! that into Anthropic content blocks, streams the answer (long edits outlast a plain request's
//! read timeout) and hands it back in the OpenAI shape.
//!
//! The assistant message it returns keeps Anthropic's own blocks under `ANTHROPIC_BLOCKS`, so the
//! next request sends them back unchanged: thinking blocks must come back exactly as they were.
//! The OpenAI adapter strips that key (see `strip_blocks`).

use anyhow::{Context, Result, bail};
use futures::StreamExt;
use serde_json::{Map, Value, json};

use crate::config::ProviderConfig;

/// The key of an assistant message that holds Anthropic's original content blocks.
pub const ANTHROPIC_BLOCKS: &str = "_anthropic";

/// Output tokens per step when the provider sets none. Opus and Sonnet think before they answer,
/// and thinking counts against this, so a whole-file edit needs room (both allow 128K).
pub const DEFAULT_MAX_TOKENS: u32 = 32_000;

/// A copy of `messages` without Anthropic's blocks, for an OpenAI-compatible server.
pub fn strip_blocks(messages: &[Value]) -> Vec<Value> {
    messages
        .iter()
        .map(|m| match m.as_object() {
            Some(o) if o.contains_key(ANTHROPIC_BLOCKS) => {
                let mut o = o.clone();
                o.remove(ANTHROPIC_BLOCKS);
                Value::Object(o)
            }
            _ => m.clone(),
        })
        .collect()
}

/// A user or assistant turn in Anthropic's shape.
fn push(out: &mut Vec<Value>, role: &str, blocks: Vec<Value>) {
    if blocks.is_empty() {
        return;
    }
    // Two turns of the same role in a row are merged (the history often has several assistant
    // lines in a row, and tool results are one user turn).
    if let Some(last) = out.last_mut()
        && last["role"] == role
        && let Some(list) = last["content"].as_array_mut()
    {
        list.extend(blocks);
        return;
    }
    out.push(json!({ "role": role, "content": blocks }));
}

fn text_block(text: &str) -> Option<Value> {
    (!text.trim().is_empty()).then(|| json!({ "type": "text", "text": text }))
}

/// A user message's content: a string, or OpenAI content parts (text, image_url data URLs).
fn user_blocks(content: &Value) -> Vec<Value> {
    match content {
        Value::String(s) => text_block(s).into_iter().collect(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| match p["type"].as_str() {
                Some("text") => text_block(p["text"].as_str().unwrap_or("")),
                Some("image_url") => {
                    let url = p["image_url"]["url"].as_str()?;
                    let (head, data) = url.strip_prefix("data:")?.split_once(";base64,")?;
                    Some(json!({ "type": "image", "source": { "type": "base64", "media_type": head, "data": data } }))
                }
                // Already an Anthropic block (ChatMessage::wire(true)).
                Some("image") => Some(p.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn assistant_blocks(m: &Value) -> Vec<Value> {
    if let Some(blocks) = m[ANTHROPIC_BLOCKS].as_array() {
        return blocks.clone();
    }
    let mut out: Vec<Value> = text_block(m["content"].as_str().unwrap_or("")).into_iter().collect();
    for call in m["tool_calls"].as_array().into_iter().flatten() {
        let input: Value = call["function"]["arguments"]
            .as_str()
            .and_then(|a| serde_json::from_str(a).ok())
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}));
        out.push(json!({ "type": "tool_use", "id": call["id"], "name": call["function"]["name"], "input": input }));
    }
    out
}

/// OpenAI-shaped messages as Anthropic's (system text, messages). The first system messages
/// become the system prompt; a later one (a digest, a note) becomes text in the user turn, so the
/// cached prefix before it stays the same. The list always starts and ends with a user turn:
/// Anthropic refuses an answer prefilled by an assistant turn.
pub fn to_anthropic(messages: &[Value]) -> (String, Vec<Value>) {
    let mut system = Vec::new();
    let mut out: Vec<Value> = Vec::new();
    for m in messages {
        match m["role"].as_str() {
            Some("system") if out.is_empty() => system.push(m["content"].as_str().unwrap_or("").to_string()),
            Some("system") => push(&mut out, "user", text_block(m["content"].as_str().unwrap_or("")).into_iter().collect()),
            Some("assistant") => {
                if out.is_empty() {
                    push(&mut out, "user", vec![json!({ "type": "text", "text": "(The conversation so far.)" })]);
                }
                push(&mut out, "assistant", assistant_blocks(m));
            }
            Some("tool") => {
                let content = match &m["content"] {
                    Value::String(s) if !s.is_empty() => s.clone(),
                    Value::String(_) | Value::Null => "(no output)".into(),
                    other => other.to_string(),
                };
                push(&mut out, "user", vec![json!({ "type": "tool_result", "tool_use_id": m["tool_call_id"], "content": content })]);
            }
            _ => push(&mut out, "user", user_blocks(&m["content"])),
        }
    }
    if out.last().is_none_or(|m| m["role"] != "user") {
        push(&mut out, "user", vec![json!({ "type": "text", "text": "Go on." })]);
    }
    (system.join("\n\n"), out)
}

/// OpenAI function tools as Anthropic tools.
pub fn to_anthropic_tools(tools: &Value) -> Vec<Value> {
    tools
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| {
            let f = &t["function"];
            let name = f["name"].as_str()?;
            let schema = if f["parameters"].is_object() { f["parameters"].clone() } else { json!({ "type": "object", "properties": {} }) };
            Some(json!({ "name": name, "description": f["description"].as_str().unwrap_or(""), "input_schema": schema }))
        })
        .collect()
}

/// The request body for one step. The provider's extra_body is merged last, so it can override
/// anything here (an effort level: `output_config = { effort = "high" }`).
pub fn request_body(p: &ProviderConfig, model: &str, messages: &[Value], tools: &Value, response_format: Option<&Value>) -> Result<Value> {
    let (system, msgs) = to_anthropic(messages);
    let mut body = json!({
        "model": model,
        "max_tokens": p.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        "messages": msgs,
        "stream": true,
        // KC-01: automatic prompt caching; every step re-sends the whole conversation.
        "cache_control": match p.ttl.as_deref() {
            Some(ttl) => json!({ "type": "ephemeral", "ttl": ttl }),
            None => json!({ "type": "ephemeral" }),
        },
    });
    if !system.is_empty() {
        body["system"] = json!(system);
    }
    let tools = to_anthropic_tools(tools);
    if !tools.is_empty() {
        body["tools"] = json!(tools);
    }
    if let Some(schema) = response_format.map(|f| &f["json_schema"]["schema"]).filter(|s| s.is_object()) {
        body["output_config"] = json!({ "format": { "type": "json_schema", "schema": schema } });
    }
    if let (Some(extra), Some(obj)) = (&p.extra_body, body.as_object_mut()) {
        for (k, v) in serde_json::to_value(extra)?.as_object().into_iter().flatten() {
            // output_config is merged, so an effort level keeps a JSON format asked for here.
            match (obj.get_mut(k).and_then(Value::as_object_mut), v.as_object()) {
                (Some(have), Some(more)) if k == "output_config" => have.extend(more.clone()),
                _ => {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
    }
    Ok(body)
}

/// The answer being put together from the stream.
#[derive(Default)]
struct Answer {
    blocks: Vec<Value>,
    /// The tool input JSON of each block, as it arrives in pieces.
    partial: Vec<String>,
    stop_reason: Option<String>,
    usage: Map<String, Value>,
}

impl Answer {
    fn event(&mut self, v: &Value) {
        match v["type"].as_str() {
            Some("message_start") => {
                if let Some(u) = v["message"]["usage"].as_object() {
                    self.usage.extend(u.clone());
                }
            }
            Some("content_block_start") => {
                self.blocks.push(v["content_block"].clone());
                self.partial.push(String::new());
            }
            Some("content_block_delta") => {
                let (Some(b), Some(p)) = (self.blocks.last_mut(), self.partial.last_mut()) else { return };
                let d = &v["delta"];
                match d["type"].as_str() {
                    Some("text_delta") => append(b, "text", d["text"].as_str()),
                    Some("thinking_delta") => append(b, "thinking", d["thinking"].as_str()),
                    Some("signature_delta") => b["signature"] = d["signature"].clone(),
                    Some("input_json_delta") => p.push_str(d["partial_json"].as_str().unwrap_or("")),
                    _ => {}
                }
            }
            Some("message_delta") => {
                if let Some(r) = v["delta"]["stop_reason"].as_str() {
                    self.stop_reason = Some(r.to_string());
                }
                if let Some(u) = v["usage"].as_object() {
                    self.usage.extend(u.clone());
                }
            }
            _ => {}
        }
    }

    /// The answer in the OpenAI shape, plus its usage (with `finish_reason`). A tool call whose
    /// input was cut off at max_tokens is dropped: running half an edit would break the file.
    fn finish(mut self) -> (Value, Value) {
        let mut blocks = Vec::new();
        let mut dropped = false;
        for (mut b, raw) in self.blocks.drain(..).zip(self.partial.drain(..)) {
            if b["type"] == "tool_use" {
                let input = if raw.trim().is_empty() { Some(json!({})) } else { serde_json::from_str::<Value>(&raw).ok().filter(Value::is_object) };
                let Some(input) = input else {
                    dropped = true;
                    continue;
                };
                b["input"] = input;
            }
            blocks.push(b);
        }
        // A dropped call is reported as a cut-off answer, so the agent says nothing ran and goes on.
        if dropped {
            self.stop_reason = Some("max_tokens".into());
        }
        let text: Vec<&str> = blocks.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect();
        let mut text = text.join("\n\n");
        let calls: Vec<Value> = blocks
            .iter()
            .filter(|b| b["type"] == "tool_use")
            .map(|b| json!({ "id": b["id"], "type": "function", "function": { "name": b["name"], "arguments": b["input"].to_string() } }))
            .collect();
        let stop = self.stop_reason.as_deref().unwrap_or("");
        if stop == "refusal" && text.trim().is_empty() {
            text = "The model declined this request.".into();
        }
        let mut msg = json!({ "role": "assistant", "content": text, ANTHROPIC_BLOCKS: blocks });
        if !calls.is_empty() {
            msg["tool_calls"] = json!(calls);
        }
        let u = &self.usage;
        let finish = match stop {
            "max_tokens" => "length",
            "tool_use" => "tool_calls",
            "refusal" => "content_filter",
            _ => "stop",
        };
        let usage = json!({
            "prompt_tokens": u.get("input_tokens"),
            "completion_tokens": u.get("output_tokens"),
            "cache_creation_input_tokens": u.get("cache_creation_input_tokens"),
            "cache_read_input_tokens": u.get("cache_read_input_tokens"),
            "finish_reason": finish,
        });
        (msg, usage)
    }
}

fn append(block: &mut Value, key: &str, piece: Option<&str>) {
    let now = block[key].as_str().unwrap_or("").to_string();
    block[key] = json!(now + piece.unwrap_or(""));
}

/// One step on Anthropic's Messages API: (assistant message, usage), as `llm::chat_full` returns them.
pub async fn chat_full(
    http: &reqwest::Client,
    p: &ProviderConfig,
    model: &str,
    messages: &[Value],
    tools: &Value,
    response_format: Option<&Value>,
) -> Result<(Value, Value)> {
    let body = request_body(p, model, messages, tools, response_format)?;
    let resp = crate::llm::authorize(http.post(crate::llm::join(&p.base_url, "v1/messages")), p)
        .json(&body)
        .send()
        .await
        .context("model request failed")?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        bail!("{} answered {status}: {}", p.name, crate::llm::readable_error(&text));
    }
    let mut answer = Answer::default();
    let mut bytes = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = bytes.next().await {
        buf.push_str(&String::from_utf8_lossy(&chunk.context("the model's answer broke off")?));
        while let Some(end) = buf.find("\n\n") {
            let event: String = buf.drain(..end + 2).collect();
            for data in event.lines().filter_map(|l| l.strip_prefix("data:")).map(str::trim) {
                let Ok(v) = serde_json::from_str::<Value>(data) else { continue };
                if v["type"] == "error" {
                    bail!("{} failed: {}", p.name, v["error"]["message"].as_str().unwrap_or("unknown error"));
                }
                answer.event(&v);
            }
        }
    }
    Ok(answer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> ProviderConfig {
        toml::from_str(r#"id = "a"
name = "Claude"
kind = "anthropic"
base_url = "https://api.anthropic.com""#)
        .unwrap()
    }

    #[test]
    fn a_tool_loop_becomes_anthropic_turns() {
        let messages = vec![
            json!({ "role": "system", "content": "You are Kompanion." }),
            json!({ "role": "assistant", "content": "Earlier answer." }),
            json!({ "role": "assistant", "content": "And another." }),
            json!({ "role": "user", "content": "Fix the file." }),
            json!({ "role": "system", "content": "Steps already taken: none." }),
            json!({ "role": "assistant", "content": "Reading it.", "tool_calls": [
                { "id": "t1", "type": "function", "function": { "name": "read_file", "arguments": "{\"path\":\"/a\"}" } },
                { "id": "t2", "type": "function", "function": { "name": "list_dir", "arguments": "{\"path\":\"/\"}" } },
            ] }),
            json!({ "role": "tool", "tool_call_id": "t1", "content": "hello" }),
            json!({ "role": "tool", "tool_call_id": "t2", "content": "" }),
        ];
        let (system, msgs) = to_anthropic(&messages);
        assert_eq!(system, "You are Kompanion.");
        let roles: Vec<&str> = msgs.iter().map(|m| m["role"].as_str().unwrap()).collect();
        assert_eq!(roles, ["user", "assistant", "user", "assistant", "user"], "alternating, starting with the user");
        assert_eq!(msgs[1]["content"].as_array().unwrap().len(), 2, "two assistant lines in one turn");
        assert_eq!(msgs[2]["content"][1]["text"], "Steps already taken: none.", "a later system message is user text");
        assert_eq!(msgs[3]["content"][1], json!({ "type": "tool_use", "id": "t1", "name": "read_file", "input": { "path": "/a" } }));
        assert_eq!(msgs[4]["content"].as_array().unwrap().len(), 2, "both tool results in one user turn");
        assert_eq!(msgs[4]["content"][1]["content"], "(no output)", "an empty result is never sent empty");
    }

    #[test]
    fn the_last_turn_is_never_the_assistant() {
        let (_, msgs) = to_anthropic(&[json!({ "role": "user", "content": "Hi" }), json!({ "role": "assistant", "content": "Hello" })]);
        assert_eq!(msgs.last().unwrap()["role"], "user");
    }

    #[test]
    fn its_own_blocks_go_back_unchanged() {
        let blocks = json!([{ "type": "thinking", "thinking": "", "signature": "sig" }, { "type": "tool_use", "id": "t", "name": "x", "input": {} }]);
        let m = json!({ "role": "assistant", "content": "", "tool_calls": [], ANTHROPIC_BLOCKS: blocks });
        let (_, msgs) = to_anthropic(&[json!({ "role": "user", "content": "Go" }), m.clone(), json!({ "role": "tool", "tool_call_id": "t", "content": "ok" })]);
        assert_eq!(msgs[1]["content"], blocks);
        assert!(strip_blocks(&[m])[0].get(ANTHROPIC_BLOCKS).is_none(), "OpenAI servers never see them");
    }

    #[test]
    fn body_has_tools_caching_and_room_to_write() {
        let tools = json!([{ "type": "function", "function": { "name": "read_file", "description": "Read", "parameters": { "type": "object", "properties": { "path": { "type": "string" } } } } }]);
        let body = request_body(&provider(), "claude-opus-5-5", &[json!({ "role": "user", "content": "Hi" })], &tools, None).unwrap();
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS);
        assert_eq!(body["cache_control"], json!({ "type": "ephemeral" }));
        assert_eq!(body["tools"][0]["name"], "read_file");
        assert_eq!(body["tools"][0]["input_schema"]["properties"]["path"]["type"], "string");
        assert!(body.get("system").is_none());
        let mut p = provider();
        p.max_tokens = Some(8000);
        let schema = json!({ "type": "json_schema", "json_schema": { "name": "plan", "schema": { "type": "object" } } });
        let body = request_body(&p, "m", &[json!({ "role": "user", "content": "Hi" })], &json!([]), Some(&schema)).unwrap();
        assert_eq!(body["max_tokens"], 8000);
        assert!(body.get("tools").is_none(), "no empty tool list");
        assert_eq!(body["output_config"]["format"]["schema"], json!({ "type": "object" }));
        p.extra_body = Some(toml::from_str("output_config = { effort = \"high\" }").unwrap());
        let body = request_body(&p, "m", &[json!({ "role": "user", "content": "Hi" })], &json!([]), Some(&schema)).unwrap();
        assert_eq!(body["output_config"]["effort"], "high");
        assert_eq!(body["output_config"]["format"]["type"], "json_schema", "an effort level keeps the format");
    }

    fn stream(events: &[Value]) -> (Value, Value) {
        let mut a = Answer::default();
        for e in events {
            a.event(e);
        }
        a.finish()
    }

    #[test]
    fn a_streamed_tool_call_comes_back_in_the_openai_shape() {
        let (msg, usage) = stream(&[
            json!({ "type": "message_start", "message": { "usage": { "input_tokens": 10, "cache_read_input_tokens": 900, "cache_creation_input_tokens": 0, "output_tokens": 1 } } }),
            json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "thinking", "thinking": "" } }),
            json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "signature_delta", "signature": "abc" } }),
            json!({ "type": "content_block_start", "index": 1, "content_block": { "type": "text", "text": "" } }),
            json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "text_delta", "text": "Editing " } }),
            json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "text_delta", "text": "now." } }),
            json!({ "type": "content_block_start", "index": 2, "content_block": { "type": "tool_use", "id": "tu", "name": "edit_file", "input": {} } }),
            json!({ "type": "content_block_delta", "index": 2, "delta": { "type": "input_json_delta", "partial_json": "{\"path\": \"/a\"," } }),
            json!({ "type": "content_block_delta", "index": 2, "delta": { "type": "input_json_delta", "partial_json": " \"old\": \"x\", \"new\": \"y\"}" } }),
            json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use" }, "usage": { "output_tokens": 120 } }),
        ]);
        assert_eq!(msg["content"], "Editing now.");
        assert_eq!(msg["tool_calls"][0]["function"]["name"], "edit_file");
        let args: Value = serde_json::from_str(msg["tool_calls"][0]["function"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(args, json!({ "path": "/a", "old": "x", "new": "y" }));
        assert_eq!(msg[ANTHROPIC_BLOCKS][0]["signature"], "abc", "thinking kept for the next request");
        assert_eq!(usage["prompt_tokens"], 10);
        assert_eq!(usage["completion_tokens"], 120);
        assert_eq!(usage["cache_read_input_tokens"], 900);
        assert_eq!(usage["finish_reason"], "tool_calls");
    }

    #[test]
    fn a_tool_call_cut_off_at_the_limit_is_dropped_and_reported() {
        let (msg, usage) = stream(&[
            json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "text", "text": "" } }),
            json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": "Writing the file." } }),
            json!({ "type": "content_block_start", "index": 1, "content_block": { "type": "tool_use", "id": "tu", "name": "write_file", "input": {} } }),
            json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "input_json_delta", "partial_json": "{\"path\": \"/a\", \"content\": \"fn ma" } }),
            json!({ "type": "message_delta", "delta": { "stop_reason": "max_tokens" }, "usage": { "output_tokens": 32000 } }),
        ]);
        assert!(msg.get("tool_calls").is_none());
        assert_eq!(msg[ANTHROPIC_BLOCKS].as_array().unwrap().len(), 1, "no half tool call left to answer");
        assert_eq!(usage["finish_reason"], "length");
    }

    #[test]
    fn broken_tool_input_is_reported_like_a_cut_off() {
        let (msg, usage) = stream(&[
            json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "tool_use", "id": "tu", "name": "edit_file", "input": {} } }),
            json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "input_json_delta", "partial_json": "{\"path\": " } }),
            json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use" } }),
        ]);
        assert!(msg.get("tool_calls").is_none());
        assert_eq!(usage["finish_reason"], "length", "never dropped in silence");
    }

    #[test]
    fn a_tool_without_arguments_still_runs() {
        let (msg, _) = stream(&[
            json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "tool_use", "id": "tu", "name": "system_info", "input": {} } }),
            json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use" } }),
        ]);
        assert_eq!(msg["tool_calls"][0]["function"]["arguments"], "{}");
    }
}
