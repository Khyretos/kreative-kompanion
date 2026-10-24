use super::*;
use sqlx::SqlitePool;

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    for q in [
        "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026'), ('u2','ana','x','2026')",
        "INSERT INTO projects (id, name, description, updated_at, user_id) VALUES ('p1','Shop','','2026','u1')",
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1','p1','Plan','2026','u1'), ('c2',NULL,'Loose','2026','u1')",
    ] {
        sqlx::query(q).execute(&db).await.unwrap();
    }
    db
}

fn state(db: SqlitePool) -> AppState {
    AppState::for_tests(toml::from_str("").unwrap(), db)
}

#[test]
fn calls_are_checked_before_a_card() {
    let ok = job(&json!({ "kind": "chat", "title": "  Logo   ideas ", "prompt": " Draw three logos " })).unwrap();
    assert_eq!(ok, json!({ "tool": NAME, "kind": "chat", "title": "Logo ideas", "prompt": "Draw three logos" }));
    assert_eq!(summary(&ok), "Start a new chat: \"Logo ideas\"");
    assert!(job(&json!({ "kind": "agent", "title": "x", "prompt": "y" })).is_err());
    assert!(job(&json!({ "kind": "task", "title": "", "prompt": "y" })).is_err());
    assert!(job(&json!({ "kind": "task", "title": "x".repeat(121), "prompt": "y" })).is_err());
    assert!(job(&json!({ "kind": "task", "title": "x", "prompt": " " })).is_err());
    assert_eq!(summary(&job(&json!({ "kind": "task", "title": "Ship it", "prompt": "y" })).unwrap()), "Add the task \"Ship it\"");
    assert_eq!(crate::pcagent::tools::summary(&ok), "Start a new chat: \"Logo ideas\"");
    assert_eq!(spec()["function"]["name"], NAME);
}

#[tokio::test]
async fn a_task_goes_into_the_chats_project_and_a_loose_chat_has_none() {
    let s = state(db().await);
    // A task needs a real description (the task list's own rule); the model is told why.
    let thin = job(&json!({ "kind": "task", "title": "Write the tests", "prompt": "tests" })).unwrap();
    assert!(perform(&s, "u1", "c1", &thin).await.unwrap_err().contains("a goal, the steps"));
    let j = job(&json!({ "kind": "task", "title": "Write the tests", "prompt": "Goal: unit tests for checkout.\n1. Cover the totals.\nDone when they pass." })).unwrap();
    let text = perform(&s, "u1", "c1", &j).await.unwrap();
    let (title, desc, project): (String, String, String) = sqlx::query_as("SELECT title, description, project_id FROM tasks WHERE user_id = 'u1'").fetch_one(&s.db).await.unwrap();
    assert_eq!((title.as_str(), desc.as_str(), project.as_str()), ("Write the tests", "Goal: unit tests for checkout.\n1. Cover the totals.\nDone when they pass.", "p1"));
    assert!(text.contains("task="), "{text}");
    let e = perform(&s, "u1", "c2", &j).await.unwrap_err();
    assert!(e.contains("not in a project"), "{e}");
    // Someone else's chat: nothing is made.
    assert!(perform(&s, "u2", "c1", &j).await.is_err());
}

#[tokio::test]
async fn a_chat_is_made_in_the_same_project_with_the_prompt_as_its_first_message() {
    let s = state(db().await);
    let j = job(&json!({ "kind": "chat", "title": "Logo ideas", "prompt": "Draw three logos for the shop." })).unwrap();
    let text = perform(&s, "u1", "c1", &j).await.unwrap();
    let id = text.split("chat=").nth(1).unwrap().split(')').next().unwrap().to_string();
    let (title, project, owner): (String, Option<String>, String) = sqlx::query_as("SELECT title, project_id, user_id FROM chats WHERE id = ?").bind(&id).fetch_one(&s.db).await.unwrap();
    assert_eq!((title.as_str(), project.as_deref(), owner.as_str()), ("Logo ideas", Some("p1"), "u1"));
    let (author, body): (String, String) = sqlx::query_as("SELECT author, text FROM messages WHERE chat_id = ?").bind(&id).fetch_one(&s.db).await.unwrap();
    assert_eq!((author.as_str(), body.as_str()), ("user", "Draw three logos for the shop."));
    assert!(text.contains("no model is set"), "{text}");
}
