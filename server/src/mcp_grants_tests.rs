//! KG-05 tests: a task step reaches an MCP tool only under a grant.
use super::*;
use crate::mcp::tests::{server, stub};
use serde_json::json;

fn g(server: &str, tool: &str) -> (String, String) {
    (server.to_string(), tool.to_string())
}

#[test]
fn a_star_grant_allows_every_tool_of_its_server_and_a_tool_grant_only_that_tool() {
    assert!(allows(&[g("blender", "*")], "blender", "render"));
    assert!(allows(&[g("blender", "render")], "blender", "render"));
    assert!(!allows(&[g("blender", "render")], "blender", "delete_scene"));
    assert!(!allows(&[g("blender", "*")], "windshift", "render"), "a grant never crosses servers");
    assert!(!allows(&[], "blender", "render"));
}

async fn state(url: &str) -> AppState {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u', 'u', 'x', '2026')").execute(&db).await.unwrap();
    // SAFETY: tests that set this variable use a name no other test reads.
    unsafe { std::env::set_var("KK_TEST_MCP_TOKEN", "secret") };
    let mut cfg: crate::config::Config = toml::from_str("").unwrap();
    cfg.mcp.push(server(url, Some("KK_TEST_MCP_TOKEN")));
    AppState::for_tests(cfg, db)
}

async fn grant(s: &AppState, server: &str, tool: &str, expires: &str) {
    sqlx::query("INSERT INTO mcp_grants (id, user_id, server, tool, granted_at, expires) VALUES (?, 'u', ?, ?, '2026', ?)")
        .bind(util::new_id()).bind(server).bind(tool).bind(expires).execute(&s.db).await.unwrap();
}

#[tokio::test]
async fn without_a_grant_no_tool_is_offered_and_a_call_is_denied() {
    let s = state(&stub().await).await;
    let (specs, map) = tools_for(&s, "u").await;
    assert!(specs.is_empty() && map.is_empty());
    // Even a model that guesses the function name gets nothing from the server.
    let guessed: HashMap<String, (String, String)> = [("stub__search".to_string(), ("stub".to_string(), "search".to_string()))].into();
    let out = call(&s, "u", &guessed, "stub__search", &json!({"q": "rust"})).await;
    assert!(out.starts_with("Denied"), "{out}");
}

#[tokio::test]
async fn under_a_grant_the_tool_is_offered_and_the_call_reaches_the_server() {
    let s = state(&stub().await).await;
    grant(&s, "stub", "*", &util::in_hours(1)).await;
    let (specs, map) = tools_for(&s, "u").await;
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0]["function"]["name"], "stub__search");
    assert_eq!(call(&s, "u", &map, "stub__search", &json!({"q": "rust"})).await, "found rust");
}

#[tokio::test]
async fn an_expired_or_other_servers_grant_does_not_count() {
    let s = state(&stub().await).await;
    grant(&s, "stub", "*", "2020-01-01T00:00:00Z").await;
    grant(&s, "other", "*", &util::in_hours(1)).await;
    assert!(tools_for(&s, "u").await.0.is_empty());
}

#[tokio::test]
async fn a_tool_grant_offers_only_that_tool_and_revoking_stops_the_call() {
    let s = state(&stub().await).await;
    grant(&s, "stub", "other_tool", &util::in_hours(1)).await;
    assert!(tools_for(&s, "u").await.0.is_empty(), "the server has no tool named other_tool");
    grant(&s, "stub", "search", &util::in_hours(1)).await;
    let (_, map) = tools_for(&s, "u").await;
    assert_eq!(call(&s, "u", &map, "stub__search", &json!({"q": "x"})).await, "found x");
    sqlx::query("DELETE FROM mcp_grants").execute(&s.db).await.unwrap();
    assert!(call(&s, "u", &map, "stub__search", &json!({"q": "x"})).await.starts_with("Denied"));
}
