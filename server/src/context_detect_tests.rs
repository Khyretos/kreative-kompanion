use super::*;
use axum::{Json, Router, routing::{get, post}};

fn provider(base: &str, kind: &str) -> ProviderConfig {
    toml::from_str(&format!("id = \"p\"\nname = \"P\"\nkind = \"{kind}\"\nbase_url = \"{base}\"\nlocal = true")).unwrap()
}

async fn serve(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

#[test]
fn the_servers_address_is_the_base_without_its_api_version() {
    assert_eq!(root("http://h:11434/v1"), "http://h:11434");
    assert_eq!(root("http://ovms:8000/v3/"), "http://ovms:8000");
    assert_eq!(root("https://api.anthropic.com"), "https://api.anthropic.com");
    assert_eq!(root("http://h/openai/v1beta"), "http://h/openai");
    assert_eq!(root("http://h:8080"), "http://h:8080");
}

#[test]
fn each_servers_answer_gives_its_context() {
    // vLLM and OpenRouter name it in the model list; a number as text counts too; a missing or silly one does not.
    let list = json!({ "data": [{ "id": "a", "max_model_len": 32768 }, { "id": "b", "context_length": "131072" }, { "id": "c" }, { "id": "d", "context_length": 12 }] });
    assert_eq!(from_models_list(&list, "a"), Some(32_768));
    assert_eq!(from_models_list(&list, "b"), Some(131_072));
    assert_eq!((from_models_list(&list, "c"), from_models_list(&list, "d"), from_models_list(&list, "nope")), (None, None, None));
    assert_eq!(from_models_list(&json!({ "error": "x" }), "a"), None);
    // llama.cpp /props.
    assert_eq!(from_props(&json!({ "default_generation_settings": { "n_ctx": 8192, "temperature": 0.8 } })), Some(8_192));
    assert_eq!(from_props(&json!({ "n_ctx": 4096 })), Some(4_096));
    assert_eq!(from_props(&json!({ "status": "ok" })), None);
    // Ollama: loaded now (":latest" is the same model), else num_ctx, else its default; not Ollama's answer: nothing.
    let ps = json!({ "models": [{ "name": "qwen3:8b", "model": "qwen3:8b", "context_length": 32768 }, { "name": "llama3:latest", "context_length": 8192 }] });
    assert_eq!((from_ollama_ps(&ps, "qwen3:8b"), from_ollama_ps(&ps, "llama3"), from_ollama_ps(&ps, "other")), (Some(32_768), Some(8_192), None));
    let show = json!({ "model_info": { "qwen3.context_length": 131072 }, "parameters": "stop \"<|im_end|>\"\nnum_ctx                        16384\ntemperature 0.7" });
    assert_eq!(from_ollama_show(&show).map(|x| x.0), Some(16_384), "the model sets it");
    let plain = json!({ "model_info": { "qwen3.context_length": 131072 }, "parameters": "stop \"x\"" });
    assert_eq!(from_ollama_show(&plain).map(|x| x.0), Some(4_096), "not the model's maximum: Ollama runs it with its default");
    assert_eq!(from_ollama_show(&json!({ "error": "not found" })), None);
    // LM Studio: only a loaded model has a context in use.
    let lm = json!({ "data": [{ "id": "a", "state": "loaded", "loaded_context_length": 16384, "max_context_length": 131072 }, { "id": "b", "state": "not-loaded", "max_context_length": 32768 }] });
    assert_eq!((from_lmstudio(&lm, "a"), from_lmstudio(&lm, "b")), (Some(16_384), None));
    assert_eq!(from_anthropic(&json!({ "id": "claude", "max_input_tokens": 200000 })), Some(200_000));
}

#[test]
fn a_refusal_names_the_limit() {
    // OpenAI, vLLM, DeepSeek.
    assert_eq!(limit_in("P answered 400 (the request was refused): This model's maximum context length is 8192 tokens. However, your messages resulted in 9100 tokens.", 0), Some(8_192));
    assert_eq!(limit_in("This model's maximum context length is 4096 tokens. However, you requested 5000 tokens (4500 in the messages, 500 in the completion).", 10), Some(4_096));
    // Mistral, Anthropic, Gemini, llama.cpp.
    assert_eq!(limit_in("Prompt contains 40000 tokens, too large for model with 32768 maximum context length", 0), Some(32_768));
    assert_eq!(limit_in("prompt is too long: 213000 tokens > 200000 maximum", 0), Some(200_000));
    assert_eq!(limit_in("The input token count (1200000) exceeds the maximum number of tokens allowed (1048576).", 0), Some(1_048_576));
    assert_eq!(limit_in(r#"{"error":{"code":400,"message":"the request exceeds the available context size","type":"exceed_context_size_error","n_prompt_tokens":5000,"n_ctx":4096}}"#, 0), Some(4_096));
    // Not about length, or not believable.
    assert_eq!(limit_in("Coder answered 500 (the service had a problem): the model is busy or out of GPU memory", 5_000), None);
    assert_eq!(limit_in("maximum context length is 12 tokens", 0), None);
    assert_eq!(limit_in("", 0), None);
}

#[test]
fn the_queue_guards_estimate_is_scaled_by_how_the_prompt_measured_against_it() {
    // Measured 2026-10-28 on Coder behind ovms-queue: 230 KB of documentation (235,520 characters) was refused as
    // "about 110829 tokens, the limit is 98000"; 150 KB of the same text was 45,794 real tokens, so about 71,400 for 230 KB.
    let n = limit_in("Request too large for the shared GPU: about 110829 tokens, the limit is 98000. Split it into smaller requests.", 235_520).unwrap();
    assert!((58_900..=59_100).contains(&n), "{n}");
    assert!(n < 62_000, "below the real limit of about 62,000 tokens for text like that");
    assert_eq!(limit_in("about 110829 tokens, the limit is 98000", 0), None, "without the prompt's length nothing can be scaled");
    assert_eq!(limit_in("about 90000 tokens, the limit is 98000", 235_520), None, "a prompt under the limit was not refused for length");
}

#[tokio::test]
async fn detect_reads_what_each_kind_of_server_offers() {
    // vLLM-like: the model list has it.
    let base = serve(Router::new().route("/v1/models", get(|| async { Json(json!({ "data": [{ "id": "m", "max_model_len": 32768 }] })) }))).await;
    let f = detect(&reqwest::Client::new(), &provider(&format!("{base}/v1"), "openai-compatible"), "m").await.unwrap();
    assert_eq!((f.tokens, f.from.as_str()), (32_768, "the server's model list"));
    // llama.cpp: /props, found next to the /v1 base.
    let base = serve(Router::new()
        .route("/v1/models", get(|| async { Json(json!({ "data": [{ "id": "m" }] })) }))
        .route("/props", get(|| async { Json(json!({ "default_generation_settings": { "n_ctx": 8192 } })) }))).await;
    let f = detect(&reqwest::Client::new(), &provider(&format!("{base}/v1"), "openai-compatible"), "m").await.unwrap();
    assert_eq!((f.tokens, f.from.as_str()), (8_192, "llama.cpp (/props)"));
    // Ollama: loaded now, else /api/show (a POST that loads nothing).
    let base = serve(Router::new()
        .route("/v1/models", get(|| async { Json(json!({ "data": [{ "id": "q" }] })) }))
        .route("/api/ps", get(|| async { Json(json!({ "models": [{ "name": "q:latest", "context_length": 32768 }] })) }))).await;
    let f = detect(&reqwest::Client::new(), &provider(&format!("{base}/v1"), "openai-compatible"), "q").await.unwrap();
    assert_eq!((f.tokens, f.from.as_str()), (32_768, "Ollama (loaded now)"));
    let base = serve(Router::new()
        .route("/v1/models", get(|| async { Json(json!({ "data": [{ "id": "q" }] })) }))
        .route("/api/ps", get(|| async { Json(json!({ "models": [] })) }))
        .route("/api/show", post(|Json(b): Json<Value>| async move {
            assert_eq!(b["model"], "q");
            Json(json!({ "model_info": {}, "parameters": "num_ctx 12288" }))
        }))).await;
    let f = detect(&reqwest::Client::new(), &provider(&format!("{base}/v1"), "openai-compatible"), "q").await.unwrap();
    assert_eq!(f.tokens, 12_288);
    // Anthropic.
    let base = serve(Router::new().route("/v1/models/claude-x", get(|| async { Json(json!({ "max_input_tokens": 200000 })) }))).await;
    let f = detect(&reqwest::Client::new(), &provider(&base, "anthropic"), "claude-x").await.unwrap();
    assert_eq!((f.tokens, f.from.as_str()), (200_000, "the Anthropic API"));
}

#[tokio::test]
async fn a_server_with_nothing_to_say_gives_nothing_and_never_gets_a_chat() {
    // Like OVMS: a model list without sizes, and no other endpoint; a chat request would fail the test.
    let chats = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = chats.clone();
    let base = serve(Router::new()
        .route("/v3/models", get(|| async { Json(json!({ "data": [{ "id": "Coder", "owned_by": "OVMS" }] })) }))
        .route("/v3/chat/completions", post(move || async move { counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst); "no" }))).await;
    let c = reqwest::Client::new();
    assert_eq!(detect(&c, &provider(&format!("{base}/v3"), "openai-compatible"), "Coder").await, None);
    assert_eq!(detect(&c, &provider("http://127.0.0.1:9/v1", "openai-compatible"), "m").await, None, "a server that is down");
    assert_eq!(detect(&c, &provider(&format!("{base}/v3"), "openai-compatible"), "a b").await, None, "a name that is not an id");
    assert_eq!(chats.load(std::sync::atomic::Ordering::SeqCst), 0, "detection never sends a chat request");
}
