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

    let (note, images) = for_model(&d, "c1", &files, "Coder", true, &mut Plan::new(None, true)).await;
    assert!(note.contains("[Attached file: notes.md]\nhello from the notes\n[End of notes.md]"), "{note}");
    assert!(note.contains("cannot see pictures") && images.is_empty(), "{note}");

    let (note, images) = for_model(&d, "c1", &files, "qwen3.5:9b", true, &mut Plan::new(None, true)).await;
    assert!(note.contains("[Attached picture: cat.png]"), "{note}");
    assert_eq!(images.len(), 1);
    assert!(images[0].starts_with("data:image/png;base64,"));
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn long_text_shows_its_start_and_says_how_to_read_the_rest() {
    let d = tmp();
    let big: String = (1..=2_000).map(|i| format!("row {i} of the log\n")).collect();
    let t = chat_files::save_bytes(&d, "c1", "txt", big.as_bytes()).await.unwrap();
    let files = vec![Attached { name: "big.log".into(), file: t, kind: "text".into() }];
    // A model with tools gets the start and the line to go on from; an unknown window counts as small (about 6,000 characters).
    let mut plan = Plan::new(None, true);
    let (note, _) = for_model(&d, "c1", &files, "Coder", true, &mut plan).await;
    assert!(note.contains("[Attached file: big.log]\nrow 1 of the log\n"), "{note}");
    assert!(note.contains("read the rest with read_attachment name=\"big.log\" from_line=") && note.contains("search_attachment"), "{}", &note[note.len() - 200..]);
    assert!(note.chars().count() < plan.budget.inline + 400 && plan.cut && plan.left < 100, "{} {}", note.len(), plan.left);
    // A larger window shows more.
    let mut big_plan = Plan::new(Some(128_000), true);
    let (more, _) = for_model(&d, "c1", &files, "Coder", true, &mut big_plan).await;
    assert!(more.len() > note.len() * 4 && !big_plan.cut && more.ends_with("[End of big.log]"), "{}", more.len());
    // A model that has no tools just gets the start and is told the rest is missing.
    let (cut, _) = for_model(&d, "c1", &files, "Coder", true, &mut Plan::new(None, false)).await;
    assert!(cut.contains("[cut here: the rest of big.log (") && !cut.contains("read_attachment"), "{}", &cut[cut.len() - 120..]);
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn the_files_of_one_prompt_share_one_budget_newest_first() {
    let d = tmp();
    // Letters that are not in the notes around the text, so they can be counted.
    let body = |c: char| c.to_string().repeat(4_000);
    let (a, b, c) = (chat_files::save_bytes(&d, "c1", "txt", body('Q').as_bytes()).await.unwrap(), chat_files::save_bytes(&d, "c1", "txt", body('R').as_bytes()).await.unwrap(), chat_files::save_bytes(&d, "c1", "txt", body('S').as_bytes()).await.unwrap());
    let mk = |name: &str, file: &String| vec![Attached { name: name.into(), file: file.clone(), kind: "text".into() }];
    let mut plan = Plan::new(None, true);
    let (n3, _) = for_model(&d, "c1", &mk("c.txt", &c), "Coder", true, &mut plan).await;
    let (n2, _) = for_model(&d, "c1", &mk("b.txt", &b), "Coder", true, &mut plan).await;
    let (n1, _) = for_model(&d, "c1", &mk("a.txt", &a), "Coder", true, &mut plan).await;
    assert!(n3.matches('S').count() == 4_000 && n3.ends_with("[End of c.txt]"), "the newest takes what it needs first, here all of it");
    assert!(n2.contains("[Attached file: b.txt]") && n2.matches('R').count() == 2_000 && n2.contains("from_line="), "the next one gets what is left and the way to read on");
    assert!(n1.contains("Its text is not shown here: read it with read_attachment name=\"a.txt\""), "{n1}");
    assert!(!n1.contains("QQQQ") && plan.left < 400 && plan.cut);
    let total = n1.len() + n2.len() + n3.len();
    assert!(total < plan.budget.inline + 1_500, "all files together stay near one budget: {total}");
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn two_files_in_one_message_split_the_room() {
    let d = tmp();
    let (a, b) = (chat_files::save_bytes(&d, "c1", "txt", "a".repeat(9_000).as_bytes()).await.unwrap(), chat_files::save_bytes(&d, "c1", "txt", b"short second file").await.unwrap());
    let files = vec![Attached { name: "a.txt".into(), file: a, kind: "text".into() }, Attached { name: "b.txt".into(), file: b, kind: "text".into() }];
    let (note, _) = for_model(&d, "c1", &files, "Coder", true, &mut Plan::new(None, true)).await;
    assert!(note.contains("short second file\n[End of b.txt]"), "the second file is not squeezed out by the first");
    assert!(note.matches('a').count() < 3_100, "the first got about half");
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn other_files_are_kept_as_they_are_with_a_text_view_next_to_them() {
    let d = tmp();
    let docx = crate::attach_docs::stored_zip(&[("word/document.xml", br#"<w:p><w:r><w:t>Hello from Word</w:t></w:r></w:p>"#)]);
    let got = receive(&d, "c1", Body::from(docx.clone()), None).await.unwrap();
    let stored = keep(&d, "c1", "letter.docx", "file", &got).await.unwrap();
    assert!(stored.ends_with(".bin") && !got.path.exists());
    assert_eq!(std::fs::read(chat_files::path_for(&d, "c1", &stored).unwrap()).unwrap(), docx, "the original is kept for download");
    let view = std::fs::read_to_string(d.join("c1").join(stored.replace(".bin", ".txt"))).unwrap();
    assert!(view.starts_with("Word document: letter.docx (") && view.contains("Hello from Word"), "{view}");
    let files = vec![Attached { name: "letter.docx".into(), file: stored, kind: "file".into() }];
    assert!(valid(&d, "c1", &files));
    let (note, images) = for_model(&d, "c1", &files, "Coder", true, &mut Plan::new(None, true)).await;
    assert!(note.contains("[Attached file: letter.docx]\nWord document: letter.docx (") && note.contains("Hello from Word") && images.is_empty(), "{note}");

    // A file that cannot be read as text is named, and its view says what it is.
    let got = receive(&d, "c1", Body::from(vec![0u8, 1, 2, 3, 250, 251]), None).await.unwrap();
    let stored = keep(&d, "c1", "blob.dat", "file", &got).await.unwrap();
    let files = vec![Attached { name: "blob.dat".into(), file: stored, kind: "file".into() }];
    let (note, _) = for_model(&d, "c1", &files, "Coder", true, &mut Plan::new(None, true)).await;
    assert!(note.contains("Binary file: blob.dat (6 B)") && note.contains("no readable text found"), "{note}");

    // A file with no view at all (an older one, or a tool's) is only named.
    let bare = chat_files::save_bytes(&d, "c1", "bin", &[1, 2, 3]).await.unwrap();
    let files = vec![Attached { name: "x.bin".into(), file: bare, kind: "file".into() }];
    let (note, _) = for_model(&d, "c1", &files, "Coder", true, &mut Plan::new(None, true)).await;
    assert!(note.contains("[Attached file: x.bin (3 B). It has no text to read"), "{note}");
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
async fn text_is_kept_whole_and_pdf_pages_are_marked() {
    let d = tmp();
    let text = "line of text\n".repeat(400_000);
    let got = receive(&d, "c1", Body::from(text.clone()), None).await.unwrap();
    let name = keep(&d, "c1", "big.log", "text", &got).await.unwrap();
    assert!(name.ends_with(".txt") && !got.path.exists(), "a text file is renamed, not copied");
    assert_eq!(std::fs::metadata(chat_files::path_for(&d, "c1", &name).unwrap()).unwrap().len(), text.len() as u64, "none of it is cut");

    let got = receive(&d, "c1", Body::from(vec![0u8, 1, 2]), None).await.unwrap();
    assert!(keep(&d, "c1", "x.log", "text", &got).await.is_err(), "binary data is not text");
    let got = receive(&d, "c1", Body::from("  \n "), None).await.unwrap();
    assert!(keep(&d, "c1", "blank.txt", "text", &got).await.unwrap_err().to_string().contains("no text found"));
    assert_eq!(attach_docs::mark_pages("a\x0cb\x0c"), "=== page 1 ===\na\n=== page 2 ===\nb\n");
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn a_picture_too_big_for_the_model_is_named_not_sent() {
    let d = tmp();
    let (stored, path) = chat_files::new_file(&d, "c1", "png").await.unwrap();
    let f = std::fs::File::create(&path).unwrap();
    f.set_len(MAX_MODEL_PICTURE + 1).unwrap();
    let files = vec![Attached { name: "huge.png".into(), file: stored, kind: "image".into() }];
    let (note, images) = for_model(&d, "c1", &files, "qwen3.5:9b", true, &mut Plan::new(None, true)).await;
    assert!(note.contains("[Attached picture: huge.png (20.0 MB)") && note.contains("too large to show you"), "{note}");
    assert!(images.is_empty());
    std::fs::remove_dir_all(&d).ok();
}
