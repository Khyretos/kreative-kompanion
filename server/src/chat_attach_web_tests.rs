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
async fn other_files_are_kept_as_they_are_and_the_model_gets_their_name_and_size() {
    let d = tmp();
    let data = vec![0u8, 1, 2, 3, 250, 251];
    let stored = chat_files::save_bytes(&d, "c1", "bin", &data).await.unwrap();
    assert!(stored.ends_with(".bin"), "{stored}");
    let files = vec![Attached { name: "scene.blend1".into(), file: stored.clone(), kind: "file".into() }];
    assert!(valid(&d, "c1", &files));
    assert_eq!(std::fs::read(chat_files::path_for(&d, "c1", &stored).unwrap()).unwrap(), data);
    let (note, images) = for_model(&d, "c1", &files, "Coder", true).await;
    assert!(note.contains("[Attached file: scene.blend1 (6 B)"), "{note}");
    assert!(note.contains("not a text document") && images.is_empty(), "{note}");
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

fn chunks(parts: Vec<Vec<u8>>) -> Body {
    Body::from_stream(futures::stream::iter(parts.into_iter().map(|p| Ok::<_, std::io::Error>(axum::body::Bytes::from(p)))))
}

#[tokio::test]
async fn an_upload_is_streamed_to_disk_and_only_the_users_setting_limits_it() {
    let d = tmp();
    let big = vec![b'a'; 3 * 1024 * 1024];
    let got = receive(&d, "c1", chunks(big.chunks(100_000).map(|c| c.to_vec()).collect()), None).await.unwrap();
    assert_eq!(got.size, big.len() as u64);
    assert_eq!(got.head.len(), SNIFF);
    assert_eq!(std::fs::metadata(&got.path).unwrap().len(), big.len() as u64);
    assert!(chat_files::path_for(&d, "c1", got.path.file_name().unwrap().to_str().unwrap()).is_none(), "a part file is never served");
    let err = receive(&d, "c1", chunks(vec![vec![1u8; 600_000], vec![1u8; 600_000]]), Some(1024 * 1024)).await.err().unwrap();
    assert!(err.to_string().contains("1 MB"), "{err}");
    assert_eq!(std::fs::read_dir(d.join("c1")).unwrap().count(), 1, "the refused upload leaves nothing behind");
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn a_big_text_file_is_kept_cut_and_any_other_file_is_renamed() {
    let d = tmp();
    let text = "line of text\n".repeat(400_000);
    let got = receive(&d, "c1", Body::from(text), None).await.unwrap();
    let name = keep(&d, "c1", "big.log", "text", &got).await.unwrap();
    let kept = std::fs::read_to_string(chat_files::path_for(&d, "c1", &name).unwrap()).unwrap();
    assert!(kept.starts_with("line of text\n") && kept.contains("[cut here: ") && kept.ends_with(" more bytes not kept]"), "{}", &kept[kept.len() - 60..]);
    assert!(kept.len() < chat_files::MAX_BYTES / 4 + 100);

    let got = receive(&d, "c1", Body::from(vec![0u8, 1, 2]), None).await.unwrap();
    let name = keep(&d, "c1", "a.zip", "file", &got).await.unwrap();
    assert!(name.ends_with(".bin") && !got.path.exists());
    assert_eq!(std::fs::read(chat_files::path_for(&d, "c1", &name).unwrap()).unwrap(), vec![0u8, 1, 2]);

    let got = receive(&d, "c1", Body::from(vec![0u8, 1, 2]), None).await.unwrap();
    assert!(keep(&d, "c1", "x.log", "text", &got).await.is_err(), "binary data is not text");
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn a_picture_too_big_for_the_model_is_named_not_sent() {
    let d = tmp();
    let (stored, path) = chat_files::new_file(&d, "c1", "png").await.unwrap();
    let f = std::fs::File::create(&path).unwrap();
    f.set_len(MAX_MODEL_PICTURE + 1).unwrap();
    let files = vec![Attached { name: "huge.png".into(), file: stored, kind: "image".into() }];
    let (note, images) = for_model(&d, "c1", &files, "qwen3.5:9b", true).await;
    assert!(note.contains("[Attached picture: huge.png (20.0 MB)") && note.contains("too large to show you"), "{note}");
    assert!(images.is_empty());
    std::fs::remove_dir_all(&d).ok();
}
