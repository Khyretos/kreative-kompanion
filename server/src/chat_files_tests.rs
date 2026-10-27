use super::*;
use base64::Engine;

const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";

fn tmp() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("kk-chatfiles-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_folder_sits_next_to_the_database() {
    assert_eq!(dir_for(std::path::Path::new("/data/kompanion.db")), std::path::PathBuf::from("/data/chat-files"));
}

#[tokio::test]
async fn saves_an_image_under_the_chat_and_finds_it_again() {
    let d = tmp();
    let name = save(&d, "chat-1", "image/png", PNG).await.unwrap();
    assert!(name.ends_with(".png") && name.len() == 40, "{name}");
    let p = path_for(&d, "chat-1", &name).unwrap();
    assert_eq!(p, d.join("chat-1").join(&name));
    let bytes = std::fs::read(&p).unwrap();
    assert_eq!(bytes, base64::engine::general_purpose::STANDARD.decode(PNG).unwrap());
    assert_eq!(content_type(&name), "image/png");
    let j = save(&d, "chat-1", "image/jpeg", PNG).await.unwrap();
    assert!(j.ends_with(".jpg"));
    assert_eq!(content_type(&j), "image/jpeg");
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn refuses_other_types_bad_data_and_odd_names() {
    let d = tmp();
    assert!(save(&d, "chat-1", "image/gif", PNG).await.is_err());
    assert!(save(&d, "chat-1", "image/png", "not base64 !!").await.is_err());
    assert!(save(&d, "../x", "image/png", PNG).await.is_err());
    let big = base64::engine::general_purpose::STANDARD.encode(vec![0u8; MAX_BYTES + 1]);
    assert!(save(&d, "chat-1", "image/png", &big).await.is_err());
    let name = save(&d, "chat-1", "image/png", PNG).await.unwrap();
    assert!(path_for(&d, "../chat-1", &name).is_none());
    assert!(path_for(&d, "chat-1", "../x.png").is_none());
    assert!(path_for(&d, "chat-1", "x.png").is_none());
    assert!(path_for(&d, "chat-1", &name.replace(".png", ".svg")).is_none());
    assert!(path_for(&d, "chat-2", &name).is_none(), "another chat's folder has no such file");
    std::fs::remove_dir_all(&d).ok();
}

#[tokio::test]
async fn keeps_any_attached_file_up_to_the_larger_limit() {
    let d = tmp();
    let name = save_bytes(&d, "chat-1", "bin", &vec![7u8; MAX_BYTES + 1]).await.unwrap();
    assert!(name.ends_with(".bin") && path_for(&d, "chat-1", &name).is_some());
    assert_eq!(content_type(&name), "application/octet-stream");
    assert!(save_bytes(&d, "chat-1", "bin", &vec![7u8; MAX_ATTACHMENT + 1]).await.is_err());
    assert!(save_bytes(&d, "chat-1", "png", &vec![7u8; MAX_BYTES + 1]).await.is_err(), "pictures keep the smaller limit");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_download_is_named_as_the_user_named_it() {
    let stored = "0b1f2c3d-0000-4000-8000-000000000001.bin";
    assert_eq!(disposition(None, stored), format!("attachment; filename=\"{stored}\"; filename*=UTF-8''{stored}"));
    assert_eq!(disposition(Some("model v2.blend"), stored), "attachment; filename=\"model v2.blend\"; filename*=UTF-8''model%20v2.blend");
    assert_eq!(disposition(Some("../../etc/pass\"wd;.zip"), stored), "attachment; filename=\"passwd.zip\"; filename*=UTF-8''passwd.zip");
    assert_eq!(disposition(Some("café.zip"), stored), "attachment; filename=\"caf_.zip\"; filename*=UTF-8''caf%C3%A9.zip");
    assert_eq!(disposition(Some("  "), stored), disposition(None, stored));
}

#[tokio::test]
async fn saves_a_blend_file_that_is_only_downloaded() {
    let d = tmp();
    let name = save(&d, "chat-1", "application/x-blender", "QkxFTkQ=").await.unwrap();
    assert!(name.ends_with(".blend"), "{name}");
    assert!(path_for(&d, "chat-1", &name).is_some());
    assert_eq!(content_type(&name), "application/octet-stream");
    assert!(save(&d, "chat-1", "text/html", "PGI+").await.is_err());
    std::fs::remove_dir_all(&d).ok();
}
