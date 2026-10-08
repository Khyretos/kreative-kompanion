use super::*;

fn tmp() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("kk-attach-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[tokio::test]
async fn documents_go_to_the_model_as_text_and_pictures_only_to_models_that_see() {
    let d = tmp();
    let t = chat_files::save_bytes(&d, "c1", "txt", b"hello from the notes").await.unwrap();
    let p = chat_files::save_bytes(&d, "c1", "png", &[137, 80, 78, 71]).await.unwrap();
    let files = vec![Attached { name: "notes.md".into(), file: t, kind: "text".into() }, Attached { name: "cat.png".into(), file: p, kind: "image".into() }];
    assert!(valid(&d, "c1", &files));
    assert!(!valid(&d, "c2", &files), "files of another chat are refused");

    let (note, images) = for_model(&d, "c1", &files, "Coder", true).await;
    assert!(note.contains("[Attached file: notes.md]\nhello from the notes\n[End of notes.md]"), "{note}");
    assert!(note.contains("cannot see pictures") && images.is_empty(), "{note}");

    let (note, images) = for_model(&d, "c1", &files, "qwen3.5:9b", true).await;
    assert!(note.contains("[Attached picture: cat.png]"), "{note}");
    assert_eq!(images.len(), 1);
    assert!(images[0].starts_with("data:image/png;base64,"));
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn long_text_is_cut_and_says_so() {
    let d = tmp();
    let big = "x".repeat(25_000);
    let t = chat_files::save_bytes(&d, "c1", "txt", big.as_bytes()).await.unwrap();
    let files = vec![Attached { name: "big.txt".into(), file: t, kind: "text".into() }];
    let (note, _) = for_model(&d, "c1", &files, "Coder", true).await;
    assert!(note.contains("[cut here: 5000 more characters not shown]"), "{}", &note[note.len() - 80..]);
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn binary_data_is_not_text() {
    assert!(text_of("a.bin.txt", &[0, 1, 2]).await.is_err());
    assert_eq!(text_of("a.rs", b"fn main() {}").await.unwrap(), "fn main() {}");
}

#[test]
fn a_message_takes_a_few_files_of_known_kinds() {
    let d = tmp();
    let many: Vec<Attached> = (0..9).map(|i| Attached { name: format!("{i}.txt"), file: "x.txt".into(), kind: "text".into() }).collect();
    assert!(!valid(&d, "c1", &many));
    assert!(!valid(&d, "c1", &[Attached { name: "a".into(), file: "../../etc/passwd".into(), kind: "text".into() }]));
    assert!(!valid(&d, "c1", &[Attached { name: "a".into(), file: "x.txt".into(), kind: "exe".into() }]));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn pictures_go_out_as_each_provider_wants_them() {
    let mut m = crate::llm::ChatMessage::text("user", "what is this?".into());
    assert_eq!(m.wire(false), serde_json::json!({"role": "user", "content": "what is this?"}));
    m.images.push("data:image/png;base64,AAAA".into());
    let o = m.wire(false);
    assert_eq!(o["content"][0]["image_url"]["url"], "data:image/png;base64,AAAA");
    assert_eq!(o["content"][1], serde_json::json!({"type": "text", "text": "what is this?"}));
    let a = m.wire(true);
    assert_eq!(a["content"][0]["source"], serde_json::json!({"type": "base64", "media_type": "image/png", "data": "AAAA"}));
}
