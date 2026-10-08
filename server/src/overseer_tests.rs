use super::*;
use sqlx::sqlite::SqlitePoolOptions;

fn sample() -> Scan {
    Scan {
        text: String::new(),
        projects: vec![("p1".into(), "Kompanion".into()), ("p2".into(), "Medabots".into())],
        tasks: vec![
            TaskRef { n: 1, id: "t-run".into(), title: "Overseer chip".into(), state: "running".into() },
            TaskRef { n: 2, id: "t-q".into(), title: "Queued thing".into(), state: "queued".into() },
        ],
    }
}

#[test]
fn task_lines_become_proposals_matched_to_projects() {
    let text = "Here is a plan.\n\nTASK: Kompanion | Add a chip | The composer shows it; done when it toggles.\n- TASK: kompanion | Settings | Name and interject toggle\n**TASK:** Robot Arena | Battle prototype | Two bots fight | with a pipe\nTASK: Medabots |  | no title, skipped\n";
    let p = parse(text, &sample());
    assert_eq!(p.tasks.len(), 3);
    assert_eq!(p.tasks[0], Proposed { project: "Kompanion".into(), project_id: Some("p1".into()), title: "Add a chip".into(), description: "The composer shows it; done when it toggles.".into() });
    assert_eq!(p.tasks[1].project_id.as_deref(), Some("p1"));
    assert_eq!(p.tasks[1].project, "Kompanion");
    assert_eq!(p.tasks[1].description, "Name and interject toggle");
    assert_eq!(p.tasks[2].project, "Robot Arena");
    assert_eq!(p.tasks[2].project_id, None);
    assert_eq!(p.tasks[2].description, "Two bots fight | with a pipe");
    assert!(p.interjections.is_empty());
}

#[test]
fn interject_lines_name_running_tasks_only() {
    let text = "INTERJECT: T1 | The API key is in .env\nINTERJECT: [T1] | Use port 8095\ninterject: T2 | queued, skipped\nINTERJECT: T9 | unknown, skipped\nINTERJECT: T1 |   \n";
    let p = parse(text, &sample());
    assert_eq!(p.interjections, vec![
        Interjection { task_id: "t-run".into(), title: "Overseer chip".into(), text: "The API key is in .env".into(), sent: false },
        Interjection { task_id: "t-run".into(), title: "Overseer chip".into(), text: "Use port 8095".into(), sent: false },
    ]);
    assert!(p.tasks.is_empty());
}

#[test]
fn plain_answers_have_no_block() {
    let p = parse("All quiet. TASK: inside a sentence does not count.", &sample());
    assert_eq!(p, Proposal::default());
    assert_eq!(block(&p), "");
}

#[test]
fn the_prompt_explains_both_line_kinds() {
    let s = sample();
    let on = prompt("Loquendo", true, &s, "2026-10-08 05:00");
    assert!(on.starts_with("You are Loquendo, the Overseer"));
    assert!(on.contains("TASK: <project name> | <task title> |"));
    assert!(on.contains("INTERJECT: T<n> |"));
    assert!(on.contains("before its next step"));
    assert!(prompt("Overseer", false, &s, "now").contains("The user decides whether to send it."));
}

#[tokio::test]
async fn the_scan_lists_what_matters_and_numbers_it() {
    let db = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'kees', 'x', '2026'), ('u2', 'other', 'x', '2026')").execute(&db).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, description, updated_at, user_id) VALUES ('p1', 'Game', 'A tiny game', '2026-10-08', 'u1'), ('p9', 'Secret', '', '2026', 'u2')").execute(&db).await.unwrap();
    for (id, title, state, step, pos) in [
        ("a", "Make menu", "running", "Step 2/4: add buttons", 1),
        ("b", "Fix crash", "needs_input", "Which save file?", 2),
        ("c", "Old work", "done", "", 3),
        ("d", "Q1", "queued", "", 4), ("e", "Q2", "queued", "", 5), ("f", "Q3", "queued", "", 6), ("g", "Q4", "queued", "", 7),
    ] {
        sqlx::query("INSERT INTO tasks (id, project_id, title, state, step, progress, position, updated_at, user_id) VALUES (?, 'p1', ?, ?, ?, 0.5, ?, '2026-10-08T04:00:00Z', 'u1')")
            .bind(id).bind(title).bind(state).bind(step).bind(pos).execute(&db).await.unwrap();
    }
    sqlx::query("INSERT INTO tasks (id, project_id, title, state, updated_at, user_id) VALUES ('z', 'p9', 'Not mine', 'running', '2026', 'u2')").execute(&db).await.unwrap();
    let s = scan(&db, "u1", 8000).await;
    assert!(s.text.contains("## Game\nA tiny game\nTasks: 1 running, 1 needs you, 4 queued, 1 done"), "{}", s.text);
    assert!(s.text.contains("- [T1] Make menu: running (50%), Step 2/4: add buttons (updated 2026-10-08T04:00)"), "{}", s.text);
    assert!(s.text.contains("- [T2] Fix crash: needs you, Which save file?"), "{}", s.text);
    assert!(s.text.contains("Recently done: Old work"));
    assert!(s.text.contains("Q3") && !s.text.contains("Q4"));
    assert!(!s.text.contains("Not mine") && !s.text.contains("Secret"));
    assert_eq!(s.tasks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["a", "b", "d", "e", "f"]);
    assert_eq!(scan(&db, "nobody", 8000).await.text, "No projects yet.");
}

#[test]
fn task_lines_without_the_keyword_still_count() {
    // The 9B sometimes drops "TASK:" and bolds the project (OVR-01 live check).
    let text = "**Medabots** | Define core robot personalities | Character profiles and combat rules.\n- Kompanion | Polish chip | Make it 26px\n| Name | State | Note |\n|---|---|---|\nA sentence with one | pipe stays text.\n";
    let p = parse(text, &sample());
    assert_eq!(p.tasks.len(), 2, "{:?}", p.tasks);
    assert_eq!(p.tasks[0], Proposed { project: "Medabots".into(), project_id: Some("p2".into()), title: "Define core robot personalities".into(), description: "Character profiles and combat rules.".into() });
    assert_eq!(p.tasks[1].project_id.as_deref(), Some("p1"));
    assert_eq!(p.tasks[1].title, "Polish chip");
}

#[tokio::test]
async fn a_project_switch_turns_the_overseer_on_for_its_chats_but_not_its_thread() {
    let db = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'kees', 'x', '2026')").execute(&db).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, updated_at, user_id, overseer) VALUES ('p1', 'Game', '2026', 'u1', 1), ('p2', 'Other', '2026', 'u1', 0)").execute(&db).await.unwrap();
    sqlx::query("INSERT INTO chats (id, project_id, title, updated_at, user_id, thread, overseer) VALUES
        ('c1', 'p1', 'a', '2026', 'u1', 0, 0), ('c2', 'p1', 'thread', '2026', 'u1', 1, 0), ('c3', 'p2', 'b', '2026', 'u1', 0, 0),
        ('c4', 'p2', 'c', '2026', 'u1', 0, 1), ('c5', NULL, 'loose', '2026', 'u1', 0, 0)").execute(&db).await.unwrap();
    for (chat, on) in [("c1", true), ("c2", false), ("c3", false), ("c4", true), ("c5", false), ("missing", false)] {
        assert_eq!(chat_on(&db, chat).await, on, "{chat}");
    }
}

#[test]
fn a_proposed_task_becomes_a_valid_new_task_body() {
    // OVR-01c: the live Create button failed with "missing field `projectId`".
    let b = serde_json::from_value::<crate::tasks::NewTask>(task_body("p1", "Write the rules", "One page"));
    assert!(b.is_ok(), "{:?}", b.err());
}
