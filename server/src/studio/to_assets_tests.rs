use super::*;
use serde_json::json;

#[test]
fn provenance_keeps_the_run_details() {
    let p = provenance("vn-portrait", "rx9070", r#"{"prompt":"a fox","seed":7}"#, r#"[{"file":"nova.safetensors","licence":"Fair AI"}]"#, Some("2026-10-07T03:00:00Z"));
    assert_eq!(p, json!({
        "source": "kompanion-studio", "workflow": "vn-portrait", "gpu": "rx9070",
        "params": {"prompt": "a fox", "seed": 7},
        "models": [{"file": "nova.safetensors", "licence": "Fair AI"}],
        "created": "2026-10-07T03:00:00Z"
    }));
}

#[test]
fn provenance_survives_bad_json() {
    let p = provenance("x", "a770", "not json", "[", None);
    assert_eq!(p["params"], Value::Null);
    assert_eq!(p["models"], json!([]));
    assert_eq!(p["created"], Value::Null);
    assert_eq!(p["source"], "kompanion-studio");
}
