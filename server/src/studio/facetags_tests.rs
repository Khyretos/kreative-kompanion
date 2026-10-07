use super::*;
use serde_json::{Value, json};

fn v(t: &[&str]) -> Vec<String> {
    t.iter().map(|s| s.to_string()).collect()
}

#[test]
fn clean_drops_negations_and_young_words() {
    assert_eq!(clean("1woman, Dark Skin\nblack hair, no glasses, without beard, young girl, , teen"), v(&["1woman", "dark skin", "black hair"]));
}

#[test]
fn skin_tags_add_the_dark_skinned_tag() {
    assert_eq!(skin_tags(&v(&["1man", "brown skin"])), v(&["1man", "brown skin", "dark-skinned male"]));
    assert_eq!(skin_tags(&v(&["1woman", "tan skin"])), v(&["1woman", "tan skin", "dark-skinned female"]));
    assert_eq!(skin_tags(&v(&["1woman", "pale skin"])), v(&["1woman", "pale skin"]));
}

#[test]
fn merge_drops_what_the_prompt_says() {
    let tags = v(&["1woman", "dark skin", "black hair", "green eyes"]);
    assert_eq!(merge_tags(&tags, "a woman with Red Hair"), "1woman, dark skin, green eyes, dark-skinned female");
    assert_eq!(merge_tags(&v(&["1man", "short beard", "glasses"]), "with a goatee"), "1man, glasses");
    let long: Vec<String> = (0..60).map(|i| format!("tag number {i}")).collect();
    assert_eq!(merge_tags(&long, "x").chars().count(), 300);
}

#[tokio::test]
async fn describe_sends_a_png_and_cleans_the_answer() {
    use axum::{Json, Router, routing::post};
    let app = Router::new().route("/v3/chat/completions", post(|Json(b): Json<Value>| async move {
        let url = b["messages"][0]["content"][1]["image_url"]["url"].as_str().unwrap_or_default().to_string();
        let content = if url.starts_with("data:image/png;base64,") { "1man, Brown Skin, no glasses, boy" } else { "wrong" };
        Json(json!({ "choices": [{ "message": { "content": content } }] }))
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v3", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let tags = describe(&crate::assets::ai::Ai::fake(&url), b"png bytes").await.unwrap();
    assert_eq!(tags, v(&["1man", "brown skin"]));
}
