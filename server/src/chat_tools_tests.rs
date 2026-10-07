use super::*;
use serde_json::json;

#[test]
fn function_names_are_safe_and_unique_per_server() {
    assert_eq!(fn_name("Stack Overflow", "search_stackoverflow"), "stack_overflow__search_stackoverflow");
    assert_eq!(fn_name("Offline Wikipedia", "zim.query"), "offline_wikipedia__zim_query");
    assert!(fn_name(&"x".repeat(50), &"y".repeat(50)).len() <= 64);
}

#[test]
fn tool_specs_map_back_to_server_and_tool() {
    let tools = vec![("Stack Overflow".to_string(), vec![json!({"name": "get_post", "description": "Read a post", "inputSchema": {"type": "object", "properties": {"id": {"type": "integer"}}}})]),
                     ("Docs".to_string(), vec![json!({"name": "list_docs"})])];
    let (specs, map) = tool_specs(&tools);
    assert_eq!(specs[0], json!({"type": "function", "function": {"name": "stack_overflow__get_post", "description": "Read a post",
        "parameters": {"type": "object", "properties": {"id": {"type": "integer"}}}}}));
    assert_eq!(specs[1]["function"]["parameters"], json!({"type": "object", "properties": {}}));
    assert_eq!(map["docs__list_docs"], ("Docs".to_string(), "list_docs".to_string()));
}

#[test]
fn a_use_line_and_the_note_for_the_answer() {
    let u = ToolUse { server: "Stack Overflow".into(), tool: "search_stackoverflow".into(), args: json!({"query": "rust borrow checker", "limit": 3}), result: "Q1: ...".into(), images: vec![], files: vec![], sources: vec![] };
    assert_eq!(use_line(&u), "*Looked up Stack Overflow: search_stackoverflow (rust borrow checker)*");
    let none = ToolUse { args: json!({}), ..u.clone() };
    assert_eq!(use_line(&none), "*Looked up Stack Overflow: search_stackoverflow*");
    let note = results_note(&[u]);
    assert!(note.starts_with("You looked these up with tools for this answer"));
    assert!(note.contains("Stack Overflow: search_stackoverflow (rust borrow checker)\nQ1: ..."));
}

#[test]
fn a_use_with_a_file_links_it_for_download() {
    let u = ToolUse { server: "Blender".into(), tool: "blender_scene".into(), args: json!({}), result: String::new(), images: vec!["/api/chats/c1/files/a.png".into()], files: vec![("/api/chats/c1/files/b.blend".into(), "scene.blend".into())], sources: vec![] };
    assert!(use_line(&u).ends_with("![Blender: blender_scene](/api/chats/c1/files/a.png)\n\n[scene.blend](/api/chats/c1/files/b.blend)"));
    assert!(results_note(&[u]).contains("Never write markdown images"));
}

#[test]
fn a_use_with_a_picture_shows_it() {
    let u = ToolUse { server: "Blender".into(), tool: "blender_render".into(), args: json!({"code": "bpy.ops.mesh.primitive_monkey_add()"}), result: "Rendered 640x480.".into(), images: vec!["/api/chats/c1/files/a.png".into()], files: vec![], sources: vec![] };
    assert_eq!(use_line(&u), "*Looked up Blender: blender_render (bpy.ops.mesh.primitive_monkey_add())*\n\n![Blender: blender_render](/api/chats/c1/files/a.png)");
    let code = ToolUse { args: json!({"code": "# a cube\nbpy.ops.mesh.primitive_cube_add()"}), images: vec![], ..u.clone() };
    assert_eq!(use_line(&code), "*Looked up Blender: blender_render (# a cube bpy.ops.mesh.primitive_cube_add())*");
    assert!(results_note(&[u.clone()]).starts_with("Your tools made the picture and files shown to the user above your answer"));
    assert!(results_note(&[code]).starts_with("You looked these up"));
}

#[test]
fn the_prompt_knows_today_and_where_it_is_installed() {
    let now = time::macros::datetime!(2026-10-07 20:45 UTC);
    let t = when_where(now, Some("kireserver"), Some("the Netherlands"), true);
    assert!(t.starts_with("Today is Wednesday 2026-10-07, 20:45 UTC. Kompanion is installed on kireserver, in the Netherlands."));
    assert!(t.contains("Web search is on"));
    let bare = when_where(now, None, None, false);
    assert!(!bare.contains("installed") && !bare.contains("Web search"));
}

#[test]
fn sources_show_what_the_answer_cites() {
    let web = Source { n: 1, name: "Reuters".into(), url: Some("https://reuters.com/a".into()), excerpt: "snippet a".into() };
    let other = Source { n: 2, name: "Blog".into(), url: Some("https://blog.example/b".into()), excerpt: "snippet b".into() };
    let kb = Source { n: 3, name: "Godot / timer.md".into(), url: None, excerpt: "Timer counts down".into() };
    let u = ToolUse { server: "Web".into(), tool: "web_search".into(), args: json!({}), result: String::new(), images: vec![], files: vec![], sources: vec![web, other, kb] };
    let block = sources_block(&[u.clone()], "He is 80 ([Reuters](https://reuters.com/a)) and a Timer counts down [Godot / timer.md](src:3).");
    let json: Value = serde_json::from_str(block.trim().strip_prefix(":::sources\n").unwrap().strip_suffix("\n:::").unwrap()).unwrap();
    assert_eq!(json.as_array().unwrap().iter().map(|x| x["n"].as_u64().unwrap()).collect::<Vec<_>>(), vec![1, 3]);
    assert_eq!(json[0]["excerpt"], "snippet a");
    assert_eq!(json[1]["url"], Value::Null);
    // Nothing cited: every consulted source is listed.
    assert_eq!(serde_json::from_str::<Value>(sources_block(&[u], "no links").trim().lines().nth(1).unwrap()).unwrap().as_array().unwrap().len(), 3);
    assert_eq!(sources_block(&[], "x"), "");
}
