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
    let u = ToolUse { server: "Stack Overflow".into(), tool: "search_stackoverflow".into(), args: json!({"query": "rust borrow checker", "limit": 3}), result: "Q1: ...".into(), images: vec![] };
    assert_eq!(use_line(&u), "*Looked up Stack Overflow: search_stackoverflow (rust borrow checker)*");
    let none = ToolUse { args: json!({}), ..u.clone() };
    assert_eq!(use_line(&none), "*Looked up Stack Overflow: search_stackoverflow*");
    let note = results_note(&[u]);
    assert!(note.starts_with("You looked these up with tools for this answer"));
    assert!(note.contains("Stack Overflow: search_stackoverflow (rust borrow checker)\nQ1: ..."));
}

#[test]
fn a_use_with_a_picture_shows_it() {
    let u = ToolUse { server: "Blender".into(), tool: "blender_render".into(), args: json!({"code": "bpy.ops.mesh.primitive_monkey_add()"}), result: "Rendered 640x480.".into(), images: vec!["/api/chats/c1/files/a.png".into()] };
    assert_eq!(use_line(&u), "*Looked up Blender: blender_render (bpy.ops.mesh.primitive_monkey_add())*\n\n![Blender: blender_render](/api/chats/c1/files/a.png)");
    let code = ToolUse { args: json!({"code": "# a cube\nbpy.ops.mesh.primitive_cube_add()"}), images: vec![], ..u.clone() };
    assert_eq!(use_line(&code), "*Looked up Blender: blender_render (# a cube bpy.ops.mesh.primitive_cube_add())*");
    assert!(results_note(&[u.clone()]).starts_with("Your tools made the picture shown to the user above your answer."));
    assert!(results_note(&[code]).starts_with("You looked these up"));
}
