use super::*;
use serde_json::json;

#[test]
fn drops_pictures_the_model_made_up() {
    let t = "Here it is.\n\n![Render](sandbox:/mnt/render.png)\n\nA red cube.";
    assert_eq!(strip_images(t, &[]), "Here it is.\n\nA red cube.");
    assert_eq!(strip_images("see ![a](http://x.test/y.png) here", &[]), "see  here");
    assert_eq!(strip_images("two ![a](u1) and ![b](u2)!", &[]), "two  and !");
}

#[test]
fn keeps_the_pictures_in_the_keep_list_and_plain_links() {
    let keep = vec!["/api/chats/c1/files/x.png".to_string()];
    let t = "![ok](/api/chats/c1/files/x.png) and ![no](/other.png) and [a link](http://x.test)";
    assert_eq!(strip_images(t, &keep), "![ok](/api/chats/c1/files/x.png) and  and [a link](http://x.test)");
    assert_eq!(strip_images("nothing here", &[]), "nothing here");
    assert_eq!(strip_images("broken ![alt](no close", &[]), "broken ![alt](no close");
}

#[test]
fn finds_embedded_files_in_a_tool_result() {
    let r = json!({"content": [
        {"type": "text", "text": "Rendered"},
        {"type": "image", "mimeType": "image/png", "data": "iVBO"},
        {"type": "resource", "resource": {"uri": "file:///tmp/work/scene.blend", "mimeType": "application/x-blender", "blob": "QkxFTkQ="}},
        {"type": "resource", "resource": {"uri": "file:///tmp/a.txt", "mimeType": "text/plain", "text": "no blob"}},
        {"type": "resource", "resource": {"mimeType": "application/zip", "blob": "UEs="}}
    ]});
    assert_eq!(tool_files(&r), vec![
        ("application/x-blender".to_string(), "QkxFTkQ=".to_string(), "scene.blend".to_string()),
        ("application/zip".to_string(), "UEs=".to_string(), "file".to_string()),
    ]);
    assert!(tool_files(&json!({})).is_empty());
}
