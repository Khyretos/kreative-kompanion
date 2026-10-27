//! IMAGE-02: `send_image` shows a picture from the computer to the user in the chat.
//! The computer reads it like `read_image`; here it is saved with the chat's files and posted as a message.
use crate::AppState;

/// (mime type, base64 data) of a `data:image/...;base64,...` url.
pub fn parse_data_url(url: &str) -> Option<(&str, &str)> {
    let (head, data) = url.strip_prefix("data:")?.split_once(";base64,")?;
    head.starts_with("image/").then_some((head, data))
}

/// The file name of a path, for the caption and the picture's title.
pub fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).filter(|n| !n.is_empty()).unwrap_or_else(|| "picture".into())
}

/// The message text: the caption (the file name when there is none) and the `:::files` block that shows the picture.
pub fn message_text(caption: &str, name: &str, saved: &str) -> String {
    let caption: String = caption.trim().chars().take(300).collect();
    let caption = if caption.is_empty() { name.to_string() } else { caption };
    let files = [crate::chat_attach::Attached { name: name.to_string(), file: saved.to_string(), kind: "image".into() }];
    format!("{caption}{}", crate::chat_attach::block(&files))
}

/// Saves the picture with the chat's files and posts it as a message. What the model is told.
pub async fn send(s: &AppState, user_id: &str, chat_id: &str, path: &str, caption: &str, url: &str) -> String {
    let Some((mime, data)) = parse_data_url(url) else { return "The step failed. The computer did not return a picture.".into() };
    if !matches!(mime, "image/png" | "image/jpeg" | "image/webp") {
        return format!("Not sent: the chat shows PNG, JPEG and WebP pictures, this one is {mime}. Convert it first, then send the copy.");
    }
    let dir = crate::chat_files::dir_for(&s.config.database);
    let saved = match crate::chat_files::save(&dir, chat_id, mime, data).await {
        Ok(f) => f,
        Err(e) => return format!("Not sent: the picture could not be saved ({e})."),
    };
    let name = file_name(path);
    super::say(s, user_id, chat_id, &message_text(caption, &name, &saved)).await;
    format!("Sent: the user now sees {name} in the chat. You did not look at it yourself (use read_image for that). Do not write image markdown and do not offer to send it again.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_image_data_url_is_a_picture() {
        assert_eq!(parse_data_url("data:image/png;base64,AAAA"), Some(("image/png", "AAAA")));
        assert_eq!(parse_data_url("data:text/html;base64,AAAA"), None);
        assert_eq!(parse_data_url("https://x/y.png"), None);
    }

    #[test]
    fn the_message_carries_the_picture_as_a_files_block_with_a_caption() {
        let text = message_text("  front view ", "a.png", "11111111-1111-4111-8111-111111111111.png");
        let (shown, files) = crate::chat_attach::split(&text);
        assert_eq!(shown, "front view");
        assert_eq!(files.len(), 1);
        assert_eq!((files[0].name.as_str(), files[0].kind.as_str()), ("a.png", "image"));
        let bare = crate::chat_attach::split(&message_text("", "b.png", "x.png")).0;
        assert_eq!(bare, "b.png", "no caption: the file name");
        assert_eq!(file_name("/media/shots/front.png"), "front.png");
        assert_eq!(file_name("/"), "picture");
    }
}
