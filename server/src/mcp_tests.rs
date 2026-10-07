use super::*;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn json_and_event_stream_answers_parse() {
    let v = parse_response("application/json", r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[]}}"#).unwrap();
    assert_eq!(v, json!({"tools": []}));
    let sse = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"ok\":true}}\n\n";
    assert_eq!(parse_response("text/event-stream", sse).unwrap(), json!({"ok": true}));
    let err = parse_response("application/json", r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"no such tool"}}"#);
    assert!(err.unwrap_err().to_string().contains("no such tool"));
}

#[test]
fn tool_results_become_text() {
    let r = json!({"content": [{"type": "text", "text": "first"}, {"type": "image", "data": "x"}, {"type": "text", "text": "second"}]});
    assert_eq!(tool_text(&r), "first\nsecond");
    assert_eq!(tool_text(&json!({"content": []})), "");
}

#[test]
fn tool_results_give_their_images() {
    let r = json!({"content": [{"type": "text", "text": "Rendered"}, {"type": "image", "mimeType": "image/png", "data": "iVBO"}, {"type": "image", "data": "nomime"}, {"type": "image", "mimeType": "image/jpeg"}]});
    assert_eq!(tool_images(&r), vec![("image/png".to_string(), "iVBO".to_string())]);
    assert!(tool_images(&json!({"content": [{"type": "text", "text": "x"}]})).is_empty());
    assert!(tool_images(&json!({})).is_empty());
}

/// A stub MCP server: answers initialize, tools/list and tools/call; 401 without the right token.
async fn stub() -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut c, _)) = l.accept().await else { break };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 65536];
                let mut n = 0;
                loop {
                    let k = c.read(&mut buf[n..]).await.unwrap_or(0);
                    if k == 0 { break; }
                    n += k;
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    if let Some(h) = text.find("\r\n\r\n") {
                        let len = text[..h].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                        if n >= h + 4 + len { break; }
                    }
                }
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let authed = text.to_ascii_lowercase().contains("authorization: bearer secret");
                let body = text.split("\r\n\r\n").nth(1).unwrap_or("");
                let req: serde_json::Value = serde_json::from_str(body).unwrap_or(json!({}));
                let id = req["id"].clone();
                let (status, ctype, out) = if !authed {
                    ("401 Unauthorized", "text/plain", "unauthorized".to_string())
                } else {
                    match req["method"].as_str().unwrap_or("") {
                        "initialize" => ("200 OK", "application/json", json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"stub","version":"1"}}}).to_string()),
                        "tools/list" => ("200 OK", "text/event-stream", format!("event: message\ndata: {}\n\n", json!({"jsonrpc":"2.0","id":id,"result":{"tools":[{"name":"search","description":"Search things","inputSchema":{"type":"object","properties":{"q":{"type":"string"}}}}]}}))),
                        "tools/call" => ("200 OK", "application/json", json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":format!("found {}", req["params"]["arguments"]["q"].as_str().unwrap_or(""))}]}}).to_string()),
                        _ => ("202 Accepted", "application/json", String::new()),
                    }
                };
                let resp = format!("HTTP/1.1 {status}\r\ncontent-type: {ctype}\r\nmcp-session-id: s1\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{out}", out.len());
                let _ = c.write_all(resp.as_bytes()).await;
            });
        }
    });
    format!("http://{addr}/mcp")
}

fn server(url: &str, token_env: Option<&str>) -> crate::config::McpServerConfig {
    crate::config::McpServerConfig { name: "stub".into(), url: url.into(), token_env: token_env.map(String::from), enabled: true, description: None }
}

#[tokio::test]
async fn lists_and_calls_tools_on_a_stub_server() {
    let url = stub().await;
    // SAFETY: tests that set this variable use a name no other test reads.
    unsafe { std::env::set_var("KK_TEST_MCP_TOKEN", "secret") };
    let http = reqwest::Client::new();
    let cfg = server(&url, Some("KK_TEST_MCP_TOKEN"));
    let tools = list_tools(&http, &cfg).await.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "search");
    let (text, pics, _files) = call_tool_full(&http, &cfg, "search", &json!({"q": "rust"})).await.unwrap();
    assert!(pics.is_empty());
    assert_eq!(text, "found rust");
    let bad = list_tools(&http, &server(&url, None)).await;
    assert!(bad.unwrap_err().to_string().contains("401"));
}
