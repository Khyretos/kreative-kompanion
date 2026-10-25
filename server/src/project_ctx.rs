//! What a project chat knows: the project and its tasks (drafted by qwen3:14b).

/// Context for a chat that belongs to a project: the project and its tasks, within `budget` characters.
pub async fn project_context(db: &sqlx::SqlitePool, project_id: &str, user_id: &str, budget: usize) -> sqlx::Result<String> {
    type P = (String, String, String, Option<String>, Option<String>, Option<String>);
    let project: Option<P> = sqlx::query_as(
        "SELECT p.name, p.description, p.about, p.repo_folder, m.name, p.dev_settings FROM projects p
         LEFT JOIN machines m ON m.id = p.repo_machine_id AND m.user_id = p.user_id WHERE p.id = ? AND p.user_id = ?",
    )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(db)
        .await?;

    let Some((name, description, about, folder, computer, dev)) = project else {
        return Ok(String::new());
    };
    // DOGFOOD: where the project lives and how it builds, so a chat works there instead of searching
    // the computer for it (a chat once ran `find ~` and stopped).
    let mut place = String::new();
    if let Some(f) = folder.filter(|f| !f.is_empty()) {
        place = format!("\nFolder: {f}{}. Work there; give shell steps that folder as cwd.", computer.map(|c| format!(" on {c}")).unwrap_or_default());
    }
    if let Some(dev) = dev.and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok()) {
        let cmds: Vec<String> = ["build", "test", "run"].iter()
            .filter_map(|k| dev[*k].as_str().filter(|c| !c.trim().is_empty()).map(|c| format!("{k}: `{c}`")))
            .collect();
        if !cmds.is_empty() {
            place.push_str(&format!("\nCommands: {}.", cmds.join(", ")));
        }
    }

    let unfinished_tasks: Vec<(String, String, String)> = sqlx::query_as("SELECT title, description, state FROM tasks WHERE project_id = ? AND user_id = ? AND state != 'done' ORDER BY position")
        .bind(project_id)
        .bind(user_id)
        .fetch_all(db)
        .await?;

    let finished_tasks: Vec<(String, String, String)> = sqlx::query_as("SELECT title, description, state FROM tasks WHERE project_id = ? AND user_id = ? AND state = 'done' ORDER BY position")
        .bind(project_id)
        .bind(user_id)
        .fetch_all(db)
        .await?;

    let mut tasks = Vec::new();
    tasks.extend(unfinished_tasks);
    tasks.extend(finished_tasks);

    let mut result = format!("Project: {}\n{}{place}\n\nTasks ({}, by order):\n", name, crate::overview::with_about(&description, &about, crate::overview::ABOUT_IN_CONTEXT), tasks.len());

    let mut total_chars = result.len();
    let mut task_count = 0;

    for (title, task_description, state) in &tasks {
        // Long descriptions end in "…" (RELIABILITY: never a silent cut).
        let mut truncated_description = task_description.chars().take(300).collect::<String>().replace('\n', " ");
        if task_description.chars().count() > 300 {
            truncated_description.push('…');
        }
        let line = format!("- [{state}] {title}\n  {truncated_description}\n");
        let line_len = line.len();

        if total_chars + line_len > budget {
            break;
        }

        result.push_str(&line);
        total_chars += line_len;
        task_count += 1;
    }

    if task_count < tasks.len() {
        result.push_str(&format!("... and {} more tasks", tasks.len() - task_count));
    }

    Ok(result)
}
