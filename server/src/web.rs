//! CHAT-02: built-in chat tools: web_search through SearXNG, read_page as plain text (public addresses only).

use std::{net::IpAddr, time::Duration};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

const MAX_PAGE: usize = 2 * 1024 * 1024;
const MAX_TEXT: usize = 6000;

pub fn html_text(html: &str) -> String {
    let mut s = html.to_string();
    
    // Remove <script>...</script> and <style>...</style> blocks (case-insensitive)
    loop {
        let lower = s.to_lowercase();
        if let Some(idx_start) = lower.find("<script") {
            if let Some(idx_end) = s[idx_start..].to_lowercase().find("</script>") {
                let end = idx_start + idx_end + "</script>".len();
                s.replace_range(idx_start..end, "");
                continue;
            }
        }
        if let Some(idx_start) = lower.find("<style") {
            if let Some(idx_end) = s[idx_start..].to_lowercase().find("</style>") {
                let end = idx_start + idx_end + "</style>".len();
                s.replace_range(idx_start..end, "");
                continue;
            }
        }
        break;
    }

    // Remove every other tag (<...>)
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            // Skip until '>'; a tag separates words.
            result.push(' ');
            while let Some(&next) = chars.peek() {
                if next == '>' {
                    chars.next();
                    break;
                } else {
                    chars.next();
                }
            }
        } else {
            result.push(c);
        }
    }
    s = result;

    // Decode entities: &amp; &lt; &gt; &quot; &#39; and &nbsp; (to a space)
    s = s.replace("&amp;", "&");
    s = s.replace("&lt;", "<");
    s = s.replace("&gt;", ">");
    s = s.replace("&quot;", "\"");
    s = s.replace("&#39;", "'");
    s = s.replace("&nbsp;", " ");

    // Collapse all whitespace runs to one space, trim
    let mut collapsed = String::new();
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                collapsed.push(' ');
                prev_space = true;
            }
        } else {
            collapsed.push(c);
            prev_space = false;
        }
    }
    collapsed.trim().to_string()
}

/// Loopback, private, link-local, shared (100.64/10), unique-local and unspecified addresses.
pub fn is_private(ip: IpAddr) -> bool {
    fn v4(a: std::net::Ipv4Addr) -> bool {
        let o = a.octets();
        a.is_private() || a.is_loopback() || a.is_link_local() || a.is_unspecified() || a.is_broadcast() || (o[0] == 100 && (o[1] & 0xc0) == 64)
    }
    match ip {
        IpAddr::V4(a) => v4(a),
        IpAddr::V6(a) => {
            let s0 = a.segments()[0];
            a.is_loopback() || a.is_unspecified() || (s0 & 0xfe00) == 0xfc00 || (s0 & 0xffc0) == 0xfe80 || a.to_ipv4_mapped().is_some_and(v4)
        }
    }
}

pub fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "web_search",
            "description": "Search the web (SearXNG). Returns titles, links and snippets.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "read_page",
            "description": "Read a web page as plain text (first 6000 characters).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {"type": "string"}
                },
                "required": ["url"]
            }
        })
    ]
}

pub async fn web_search(http: &reqwest::Client, base: &str, query: &str) -> Result<String> {
    Ok(web_search_from(http, base, query, 1).await?.0)
}

/// Results numbered from `first_n`, plus (title, url, snippet) of each for the sources list (CHAT-04).
pub async fn web_search_from(http: &reqwest::Client, base: &str, query: &str, first_n: usize) -> Result<(String, Vec<(String, String, String)>)> {
    let url = format!("{}/search", base.trim_end_matches('/'));
    let resp = http.get(&url)
        .query(&[("q", query), ("format", "json")])
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .context("failed to search")?;
    
    if !resp.status().is_success() {
        bail!("search failed with {}", resp.status());
    }
    
    let json_val: Value = resp.json().await.context("invalid JSON")?;
    let results = json_val["results"].as_array().ok_or_else(|| anyhow::anyhow!("no results array"))?;
    
    if results.is_empty() {
        return Ok(("No results.".to_string(), Vec::new()));
    }

    let mut parts: Vec<String> = Vec::new();
    let mut found = Vec::new();
    for (i, item) in results.iter().take(8).enumerate() {
        let title = item["title"].as_str().unwrap_or("").to_string();
        let url_str = item["url"].as_str().unwrap_or("").to_string();
        let content = item["content"].as_str().unwrap_or("").to_string();
        
        parts.push(format!("{}. {title}\n{url_str}\n{content}", first_n + i).trim_end().to_string());
        found.push((title, url_str, content));
    }

    Ok((parts.join("\n\n"), found))
}

pub async fn read_page(url: &str, allow_private: bool) -> Result<String> {
    let mut url = reqwest::Url::parse(url).context("not a URL")?;
    
    for _ in 0..4 {
        if !matches!(url.scheme(), "http" | "https") {
            bail!("only http and https pages can be read");
        }
        
        let host = url.host_str().context("no host")?.to_string();
        let port = url.port_or_known_default().unwrap_or(80);
        
        let mut addrs = tokio::net::lookup_host((host.as_str(), port)).await?;
        let addr = addrs.next().context("the host has no address")?;
        
        if !allow_private && is_private(addr.ip()) {
            bail!("{host} is a private address");
        }

        // Connect to exactly the address that was checked (no second DNS answer).
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .resolve(&host, addr)
            .build()?;
        
        let resp = client.get(url.clone())
            .header("User-Agent", "Kompanion")
            .send()
            .await?;
        
        if resp.status().is_redirection() {
            let next = resp.headers().get("location")
                .and_then(|v| v.to_str().ok())
                .context("a redirect without a location")?;
            url = url.join(next)?;
            continue;
        }
        
        if !resp.status().is_success() {
            bail!("{} answered {}", host, resp.status());
        }
        
        let ctype = resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        
        let bytes = resp.bytes().await?;
        let body = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_PAGE)]).to_string();
        
        let text = if ctype.contains("html") {
            html_text(&body)
        } else {
            body.split_whitespace().collect::<Vec<_>>().join(" ")
        };
        
        return Ok(text.chars().take(MAX_TEXT).collect());
    }
    
    bail!("too many redirects")
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod tests;
