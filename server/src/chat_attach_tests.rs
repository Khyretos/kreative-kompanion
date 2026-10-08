use super::*;

fn a(name: &str, file: &str, kind: &str) -> Attached {
    Attached { name: name.into(), file: file.into(), kind: kind.into() }
}

#[test]
fn block_is_empty_without_files_and_round_trips_with_them() {
    assert_eq!(block(&[]), "");
    let files = vec![a("cat.png", "0b1f2c3d-0000-4000-8000-000000000001.png", "image"), a("notes \"v2\".md", "0b1f2c3d-0000-4000-8000-000000000002.txt", "text")];
    let stored = format!("Look at these{}", block(&files));
    assert!(stored.starts_with("Look at these\n\n:::files\n"), "{stored}");
    let (text, back) = split(&stored);
    assert_eq!(text, "Look at these");
    assert_eq!(back.len(), 2);
    assert_eq!(back[1].name, "notes \"v2\".md");
    assert_eq!(back[0].kind, "image");
}

#[test]
fn split_leaves_plain_and_broken_text_alone() {
    assert_eq!(split("hello"), ("hello".to_string(), vec![]));
    let broken = "hi\n\n:::files\nnot json";
    let (text, files) = split(broken);
    assert_eq!(text, broken);
    assert!(files.is_empty());
}

#[test]
fn kinds_follow_the_extension() {
    assert_eq!(kind_for("Cat.PNG"), Some("image"));
    assert_eq!(kind_for("a.jpeg"), Some("image"));
    assert_eq!(kind_for("a.webp"), Some("image"));
    assert_eq!(kind_for("report.pdf"), Some("text"));
    assert_eq!(kind_for("main.rs"), Some("text"));
    assert_eq!(kind_for("notes.md"), Some("text"));
    assert_eq!(kind_for("data.csv"), Some("text"));
    assert_eq!(kind_for("a.gif"), None);
    assert_eq!(kind_for("tool.exe"), None);
    assert_eq!(kind_for("noextension"), None);
}

#[test]
fn only_vision_models_get_pictures() {
    for m in ["qwen3.5:9b-q8_0-64k", "Qwen2.5-VL-7B", "llava:13b", "gemma3:12b", "claude-sonnet-5-5", "gpt-4o", "pixtral-12b", "llama3.2-vision"] {
        assert!(sees_pictures(m), "{m}");
    }
    for m in ["Coder", "Autocomplete", "deepseek-chat", "llama3.1:8b"] {
        assert!(!sees_pictures(m), "{m}");
    }
}
