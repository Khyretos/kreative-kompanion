use super::*;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn html_becomes_readable_text() {
    let page = "<html><head><title>T</title><style>p{color:red}</style><script>var x=1;</script></head><body><h1>Hello</h1>\n<p>One &amp; two</p><p>three&nbsp;four</p></body></html>";
    assert_eq!(html_text(page), "T Hello One & two three four");
}

#[test]
fn private_addresses_are_recognised() {
    for a in ["127.0.0.1", "10.1.2.3", "172.16.1.26", "192.168.178.80", "169.254.1.1", "0.0.0.0", "::1", "fd00::1", "fe80::1"] {
        assert!(is_private(a.parse().unwrap()), "{a}");
    }
    for a in ["1.1.1.1", "140.82.112.3", "2606:4700::1111"] {
        assert!(!is_private(a.parse().unwrap()), "{a}");
    }
}

#[test]
fn the_tools_have_schemas() {
    let t = tools();
    assert_eq!(t.iter().map(|x| x["name"].as_str().unwrap()).collect::<Vec<_>>(), vec!["web_search", "read_page"]);
    assert_eq!(t[0]["inputSchema"]["required"], json!(["query"]));
    assert_eq!(t[1]["inputSchema"]["required"], json!(["url"]));
}

/// Serves one fixed body for every request (enough for SearXNG and a page).
async fn stub(ctype: &'static str, body: String) -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut c, _)) = l.accept().await else { break };
            let body = body.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = c.read(&mut buf).await;
                let resp = format!("HTTP/1.1 200 OK\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                let _ = c.write_all(resp.as_bytes()).await;
            });
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn searches_through_searxng() {
    let body = json!({"results": [
        {"title": "Rust 1.90 released", "url": "https://blog.rust-lang.org/x", "content": "The Rust team is happy..."},
        {"title": "Second", "url": "https://example.org/2", "content": "More"}
    ]}).to_string();
    let base = stub("application/json", body).await;
    let text = web_search(&reqwest::Client::new(), &base, "rust release").await.unwrap();
    assert_eq!(text, "1. Rust 1.90 released\nhttps://blog.rust-lang.org/x\nThe Rust team is happy...\n\n2. Second\nhttps://example.org/2\nMore");
}

#[tokio::test]
async fn reads_a_page_but_not_a_private_one_unless_allowed() {
    let base = stub("text/html", "<html><body><p>Hi there</p></body></html>".to_string()).await;
    let url = format!("{base}/page");
    let refused = read_page(&url, false).await;
    assert!(refused.unwrap_err().to_string().contains("private"));
    assert_eq!(read_page(&url, true).await.unwrap(), "Hi there");
    assert!(read_page("file:///etc/passwd", true).await.unwrap_err().to_string().contains("http"));
}
