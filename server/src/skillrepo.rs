//! Skill cards are saved as git commits through the Forgejo HTTP API.

use base64::Engine;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;

pub const DEFAULT_REPO: &str = "khyretos/kreative-kompanion";
pub const DEFAULT_PRIVATE_REPO: &str = "khyretos/kompas-skills-kees";
pub const DEFAULT_FORGE_URL: &str = "https://git.kreative-kompas.com";

/// Returns the repo ("owner/name") and the path prefix of a layer.
pub fn target(layer: &str) -> Option<(String, String)> {
    let repo = env::var("KOMPANION_SKILLS_REPO")
        .unwrap_or_else(|_| DEFAULT_REPO.to_string());
    let private_repo = env::var("KOMPANION_SKILLS_PRIVATE_REPO")
        .unwrap_or_else(|_| DEFAULT_PRIVATE_REPO.to_string());

    match layer {
        "general" => Some((repo, "skills/general/".to_string())),
        "kompanion" => Some((repo, "skills/".to_string())),
        "private" => Some((private_repo, "".to_string())),
        _ => None,
    }
}

/// True when file is not empty, ends with ".md", does not start with '/', and contains neither ".." nor '\\'.
pub fn safe_file(file: &str) -> bool {
    if file.is_empty() {
        return false;
    }
    if !file.ends_with(".md") {
        return false;
    }
    if file.starts_with('/') {
        return false;
    }
    if file.contains("..") || file.contains('\\') {
        return false;
    }
    true
}

/// A lesson is its first line plus the lines right after it that start with a space or a tab and are not blank. Returns (text without that lesson, the lesson's lines joined with '\n', no trailing newline); None when no line's trim() equals first.trim() or first.trim() is empty.
pub fn take_lesson(text: &str, first: &str) -> Option<(String, String)> {
    let first = first.trim();
    if first.is_empty() {
        return None;
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let start = lines.iter().position(|l| l.trim() == first)?;
    let end = start + 1 + lines[start + 1..]
        .iter()
        .take_while(|l| l.starts_with([' ', '\t']) && !l.trim().is_empty())
        .count();
    let lesson = lines[start..end].concat().trim_end_matches('\n').to_string();
    let rest = [&lines[..start], &lines[end..]].concat().concat();
    Some((rest, lesson))
}

/// text, a newline added first if text is not empty and does not end with one, then line.trim_end() and "\n".
pub fn append_line(text: &str, line: &str) -> String {
    let mut result = text.to_string();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result.push_str(line.trim_end());
    result.push('\n');
    result
}

#[derive(Clone)]
pub struct Forge {
    pub base: String,
    pub token: String,
    pub branch: String,
    client: Client,
}

impl Forge {
    /// base without a trailing '/': trim it.
    pub fn new(base: &str, token: &str, branch: &str) -> Forge {
        let base = base.trim_end_matches('/');
        Forge {
            base: base.to_string(),
            token: token.to_string(),
            branch: branch.to_string(),
            client: Client::new(),
        }
    }

    /// base from env KOMPANION_FORGE_URL (default "https://git.kreative-kompas.com"), token from env KOMPANION_FORGE_TOKEN (None when unset or empty), branch from KOMPANION_SKILLS_BRANCH (default "main").
    pub fn from_env() -> Option<Forge> {
        let base = env::var("KOMPANION_FORGE_URL").ok().filter(|b| !b.is_empty()).unwrap_or_else(|| DEFAULT_FORGE_URL.to_string());
        let token = env::var("KOMPANION_FORGE_TOKEN").ok().filter(|t| !t.is_empty())?;
        let branch = env::var("KOMPANION_SKILLS_BRANCH").ok().filter(|b| !b.is_empty()).unwrap_or_else(|| "main".to_string());
        Some(Forge::new(&base, &token, &branch))
    }

    /// GET {base}/api/v1/repos/{repo}/contents/{path}?ref={branch}; 404 -> Ok(None); success -> Some((sha, text)) where text is the JSON field "content" base64-decoded after removing '\n' characters; any other status -> Err(format!("forge {status}: {body}")).
    pub async fn read(&self, repo: &str, path: &str) -> Result<Option<(String, String)>, String> {
        let url = format!("{}/api/v1/repos/{}/contents/{}?ref={}", self.base, repo, path, self.branch);
        let resp = self.client.get(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if status.as_u16() == 404 {
            return Ok(None);
        }

        if !status.is_success() {
            return Err(format!("forge {}: {}", status.as_u16(), body));
        }

        let json: Value = serde_json::from_str(&body).map_err(|e| format!("forge parse: {}", e))?;
        let content_b64 = json["content"].as_str().ok_or("forge missing content")?;
        let content = content_b64.replace('\n', "");
        let decoded = base64::engine::general_purpose::STANDARD.decode(content).map_err(|e| format!("forge decode: {}", e))?;
        let sha = json["sha"].as_str().ok_or("forge missing sha")?.to_string();
        Ok(Some((sha, String::from_utf8_lossy(&decoded).to_string())))
    }

    /// calls self.read first; when the file exists, PUT {base}/api/v1/repos/{repo}/contents/{path} with JSON {"content": base64(text), "sha": sha, "message": message, "branch": branch, "author": {"name": author, "email": "kompanion@localhost"}}; when it is missing, POST to the same URL with the same JSON without "sha". Returns the JSON field commit.sha of the answer; a status other than 200/201 -> Err as in read.
    pub async fn commit(&self, repo: &str, path: &str, text: &str, message: &str, author: &str) -> Result<String, String> {
        let existing = self.read(repo, path).await?;
        let sha = existing.as_ref().and_then(|(s, _)| Some(s.clone()));

        let payload = if let Some(s) = &sha {
            json!({
                "content": base64::engine::general_purpose::STANDARD.encode(text),
                "sha": s,
                "message": message,
                "branch": self.branch,
                "author": { "name": author, "email": "kompanion@localhost" }
            })
        } else {
            json!({
                "content": base64::engine::general_purpose::STANDARD.encode(text),
                "message": message,
                "branch": self.branch,
                "author": { "name": author, "email": "kompanion@localhost" }
            })
        };

        let url = format!("{}/api/v1/repos/{}/contents/{}", self.base, repo, path);
        let method = if sha.is_some() { reqwest::Method::PUT } else { reqwest::Method::POST };
        let resp = self.client.request(method, &url)
            .header("Authorization", format!("token {}", self.token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("forge {}: {}", status.as_u16(), body));
        }

        let json: Value = serde_json::from_str(&body).map_err(|e| format!("forge parse: {}", e))?;
        let sha = json["commit"]["sha"].as_str().ok_or("forge missing commit.sha")?.to_string();
        Ok(sha)
    }

    /// GET {base}/api/v1/repos/{repo}/commits?path={path}&sha={branch}&limit=20&stat=false; maps each item to json!({"sha": item.sha, "message": first line of item.commit.message, "author": item.commit.author.name, "date": item.commit.author.date}).
    pub async fn history(&self, repo: &str, path: &str) -> Result<Vec<Value>, String> {
        let url = format!(
            "{}/api/v1/repos/{}/commits?path={}&sha={}&limit=20&stat=false",
            self.base, repo, path, self.branch
        );
        let resp = self.client.get(&url).header("Authorization", format!("token {}", self.token)).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(format!("forge {}: {}", status.as_u16(), body));
        }

        let json: Value = serde_json::from_str(&body).map_err(|e| format!("forge parse: {}", e))?;
        let items = json.as_array().ok_or("forge: commits are not a list")?;

        let mut result = Vec::new();
        for item in items {
            let sha = item["sha"].as_str().ok_or("forge missing sha")?.to_string();
            let msg = item["commit"]["message"].as_str().ok_or("forge missing message")?;
            let first_line = msg.lines().next().ok_or("forge empty message")?.to_string();
            let author_name = item["commit"]["author"]["name"].as_str().ok_or("forge missing author name")?;
            let author_date = item["commit"]["author"]["date"].as_str().ok_or("forge missing author date")?;

            result.push(json!({
                "sha": sha,
                "message": first_line,
                "author": author_name,
                "date": author_date
            }));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::{Path, State}, http::StatusCode, routing::get, Json, Router};
    use base64::Engine;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::future::IntoFuture;
    use std::sync::{Arc, Mutex};

    type Files = Arc<Mutex<HashMap<String, String>>>;

    #[test]
    fn test_target() {
        // Defaults only
        assert_eq!(target("general"), Some(("khyretos/kreative-kompanion".to_string(), "skills/general/".to_string())));
        assert_eq!(target("kompanion"), Some(("khyretos/kreative-kompanion".to_string(), "skills/".to_string())));
        assert_eq!(target("private"), Some(("khyretos/kompas-skills-kees".to_string(), "".to_string())));
        assert_eq!(target("unknown"), None);
    }

    #[test]
    fn test_safe_file() {
        assert!(safe_file("shared/git.md"));
        assert!(!safe_file(""));
        assert!(!safe_file("../x.md"));
        assert!(!safe_file("/x.md"));
        assert!(!safe_file("x.txt"));
    }

    #[test]
    fn test_take_lesson() {
        let text = "1. a\n   more a\n2. b\n";
        let result = take_lesson(text, "1. a");
        assert_eq!(result, Some(("2. b\n".to_string(), "1. a\n   more a".to_string())));

        let result = take_lesson(text, "2. b");
        assert_eq!(result, Some(("1. a\n   more a\n".to_string(), "2. b".to_string())));

        let result = take_lesson(text, "3. c");
        assert_eq!(result, None);

        let result = take_lesson(text, "");
        assert_eq!(result, None);
    }

    #[test]
    fn test_append_line() {
        let result = append_line("", "a");
        assert_eq!(result, "a\n");

        let result = append_line("a", "b");
        assert_eq!(result, "a\nb\n");

        let result = append_line("a\n", "b");
        assert_eq!(result, "a\nb\n");
    }

    #[test]
    fn test_take_lesson_empty_first() {
        let text = "1. a\n   more a\n";
        let result = take_lesson(text, "");
        assert_eq!(result, None);
    }

    #[test]
    fn test_take_lesson_no_match() {
        let text = "1. a\n   more a\n";
        let result = take_lesson(text, "2. b");
        assert_eq!(result, None);
    }

    #[test]
    fn test_take_lesson_single_line() {
        let text = "single line\n";
        let result = take_lesson(text, "single line");
        assert_eq!(result, Some((String::new(), "single line".to_string())));
    }

    #[test]
    fn test_append_line_with_newline() {
        let result = append_line("a\n", "b");
        assert_eq!(result, "a\nb\n");
    }

    #[test]
    fn test_append_line_without_newline() {
        let result = append_line("a", "b");
        assert_eq!(result, "a\nb\n");
    }

    #[tokio::test]
    async fn test_forge_stub() {
        let files: Files = Arc::default();
        let app = Router::new()
            .route("/api/v1/repos/{owner}/{name}/contents/{*path}", get(get_contents).post(post_content).put(put_content))
            .route("/api/v1/repos/{owner}/{name}/commits", get(get_commits))
            .with_state(files.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(axum::serve(listener, app).into_future());

        let forge = Forge::new(&format!("http://{addr}/"), "fake-token", "main");
        assert_eq!(forge.commit("o/r", "shared/x.md", "one", "Add x", "kees").await.unwrap(), "c1");
        assert_eq!(forge.commit("o/r", "shared/x.md", "two", "Edit x", "kees").await.unwrap(), "c1");
        assert_eq!(forge.read("o/r", "shared/x.md").await.unwrap(), Some(("s1".to_string(), "two".to_string())));
        assert_eq!(forge.read("o/r", "shared/y.md").await.unwrap(), None);

        let history = forge.history("o/r", "shared/x.md").await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0]["message"], "Edit x");
        assert_eq!(history[0]["author"], "kees");
        handle.abort();
    }

    async fn get_contents(State(files): State<Files>, Path((_, _, path)): Path<(String, String, String)>) -> Result<(StatusCode, Json<Value>), String> {
        match files.lock().unwrap().get(&path) {
            Some(text) => Ok((StatusCode::OK, Json(json!({"sha": "s1", "content": base64::engine::general_purpose::STANDARD.encode(text)})))),
            None => Ok((StatusCode::NOT_FOUND, Json(json!({"message": "not found"})))),
        }
    }

    fn store(files: &Files, path: String, body: &Value) {
        let bytes = base64::engine::general_purpose::STANDARD.decode(body["content"].as_str().unwrap_or_default()).unwrap();
        files.lock().unwrap().insert(path, String::from_utf8(bytes).unwrap());
    }

    async fn post_content(State(files): State<Files>, Path((_, _, path)): Path<(String, String, String)>, Json(body): Json<Value>) -> Result<(StatusCode, Json<Value>), String> {
        store(&files, path, &body);
        Ok((StatusCode::CREATED, Json(json!({"commit": {"sha": "c1"}}))))
    }

    async fn put_content(State(files): State<Files>, Path((_, _, path)): Path<(String, String, String)>, Json(body): Json<Value>) -> Result<(StatusCode, Json<Value>), String> {
        if body.get("sha").is_none() {
            return Ok((StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"message": "sha missing"}))));
        }
        store(&files, path, &body);
        Ok((StatusCode::OK, Json(json!({"commit": {"sha": "c1"}}))))
    }

    async fn get_commits() -> Result<Json<Value>, String> {
        Ok(Json(json!([{"sha": "c1", "commit": {"message": "Edit x\n\nmore", "author": {"name": "kees", "date": "2026-10-06T10:00:00Z"}}}])))
    }
}
