use super::*;

async fn state() -> AppState {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at, is_admin) VALUES ('a','admin','x','2026',1), ('u','user','x','2026',0)")
        .execute(&db)
        .await
        .unwrap();
    let cfg = "database = \"/srv/kompanion/kompanion.db\"\n[[provider]]\nid = \"ovms\"\nname = \"OVMS\"\nkind = \"openai-compatible\"\nbase_url = \"http://ovms:8000/v3\"\nlocal = true\n";
    AppState::for_tests(toml::from_str(cfg).unwrap(), db)
}

fn user(id: &str) -> User {
    User { id: id.into(), name: id.into() }
}

fn form(v: Value) -> Json<Form> {
    Json(serde_json::from_value(v).unwrap())
}

fn grok() -> Value {
    json!({ "name": "xAI Grok", "kind": "openai-compatible", "baseUrl": "https://api.x.ai/v1/", "apiKeyEnv": "XAI_API_KEY", "contextWindow": 256000 })
}

#[tokio::test]
async fn an_admin_adds_a_provider_that_the_whole_server_then_uses() {
    let s = state().await;
    let (code, Json(p)) = create(State(s.clone()), Extension(user("a")), form(grok())).await.unwrap();
    assert_eq!(code, StatusCode::CREATED);
    assert_eq!((p["id"].as_str(), p["baseUrl"].as_str(), p["apiKeyEnv"].as_str()), (Some("xai-grok"), Some("https://api.x.ai/v1"), Some("XAI_API_KEY")));
    // Never a key, only whether one is there.
    assert!(p.get("apiKey").is_none() && p["hasKey"].is_boolean());
    let found = s.provider("xai-grok").unwrap();
    assert_eq!((found.name.as_str(), found.context_window), ("xAI Grok", Some(256_000)));
    assert_eq!(s.providers().iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["ovms", "xai-grok"]);
    // The same name again gets its own id; the config file's providers keep theirs.
    let (_, Json(again)) = create(State(s.clone()), Extension(user("a")), form(grok())).await.unwrap();
    assert_eq!(again["id"], "xai-grok-2");
    let (_, Json(clash)) = create(State(s.clone()), Extension(user("a")), form(json!({ "name": "OVMS", "kind": "openai-compatible", "baseUrl": "http://x:1/v1", "local": true }))).await.unwrap();
    assert_eq!(clash["id"], "ovms-2");
    // A restart reads them back.
    s.custom_providers.write().unwrap().clear();
    load(&s).await;
    assert!(s.provider("xai-grok-2").is_some());
}

#[tokio::test]
async fn only_admins_and_only_key_places_meant_for_keys() {
    let s = state().await;
    assert!(matches!(create(State(s.clone()), Extension(user("u")), form(grok())).await.unwrap_err(), ApiError::Forbidden(_)));
    assert!(matches!(list(State(s.clone()), Extension(user("u"))).await.unwrap_err(), ApiError::Forbidden(_)));
    let bad = |patch: Value| {
        let mut v = grok();
        v.as_object_mut().unwrap().extend(patch.as_object().unwrap().clone());
        v
    };
    for patch in [
        json!({ "apiKeyEnv": "DATABASE_URL" }),
        json!({ "apiKeyEnv": "xai_api_key" }),
        json!({ "apiKeyEnv": null, "apiKeyFile": "/etc/shadow" }),
        json!({ "apiKeyEnv": null, "apiKeyFile": "/run/secrets/../../etc/passwd" }),
        json!({ "apiKeyEnv": null, "apiKeyFile": "secrets/key" }),
        json!({ "apiKeyFile": "/run/secrets/xai" }),
        json!({ "baseUrl": "ftp://api.x.ai" }),
        json!({ "baseUrl": "api.x.ai" }),
        json!({ "kind": "gemini" }),
        json!({ "name": " " }),
        json!({ "maxTokens": 0 }),
        json!({ "contextWindow": 10 }),
    ] {
        let r = create(State(s.clone()), Extension(user("a")), form(bad(patch.clone()))).await;
        assert!(matches!(r, Err(ApiError::BadRequest(_))), "{patch} -> {r:?}");
    }
    // Docker secrets and secrets/ next to the database are fine.
    for file in ["/run/secrets/xai_api_key", "/srv/kompanion/secrets/xai"] {
        let r = create(State(s.clone()), Extension(user("a")), form(bad(json!({ "apiKeyEnv": null, "apiKeyFile": file }))));
        assert!(r.await.is_ok(), "{file}");
    }
    let Json(l) = list(State(s.clone()), Extension(user("a"))).await.unwrap();
    assert_eq!(l["secretDirs"], json!(["/run/secrets", "/srv/kompanion/secrets"]));
    assert_eq!(l["providers"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn changes_and_removal_but_not_of_a_used_one_or_the_config_files() {
    let s = state().await;
    create(State(s.clone()), Extension(user("a")), form(grok())).await.unwrap();
    let mut v = grok();
    v["name"] = json!("Grok");
    v["maxTokens"] = json!(8000);
    let Json(p) = update(State(s.clone()), Extension(user("a")), Path("xai-grok".into()), form(v.clone())).await.unwrap();
    assert_eq!((p["name"].as_str(), p["maxTokens"].as_u64()), (Some("Grok"), Some(8000)));
    assert_eq!(s.provider("xai-grok").unwrap().max_tokens, Some(8000));
    assert!(update(State(s.clone()), Extension(user("a")), Path("ovms".into()), form(v.clone())).await.is_err());
    assert!(matches!(update(State(s.clone()), Extension(user("a")), Path("nope".into()), form(v)).await.unwrap_err(), ApiError::NotFound));
    sqlx::query("INSERT INTO user_roles (user_id, role, provider_id, model_id) VALUES ('u', 'worker', 'xai-grok', 'grok-4')").execute(&s.db).await.unwrap();
    let e = delete(State(s.clone()), Extension(user("a")), Path("xai-grok".into())).await.unwrap_err();
    assert!(format!("{e}").contains("worker"), "{e}");
    sqlx::query("DELETE FROM user_roles").execute(&s.db).await.unwrap();
    assert_eq!(delete(State(s.clone()), Extension(user("a")), Path("xai-grok".into())).await.unwrap(), StatusCode::NO_CONTENT);
    assert!(s.provider("xai-grok").is_none());
}

#[test]
fn a_key_file_is_read_and_trimmed() {
    let dir = std::env::temp_dir().join(format!("kk-key-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("key");
    std::fs::write(&f, "  test-value-not-a-secret\n").unwrap();
    let p: ProviderConfig = toml::from_str(&format!("id = \"x\"\nname = \"X\"\nkind = \"anthropic\"\nbase_url = \"https://x\"\napi_key_file = \"{}\"", f.display())).unwrap();
    assert_eq!(p.api_key().as_deref(), Some("test-value-not-a-secret"));
    std::fs::remove_dir_all(&dir).unwrap();
}
