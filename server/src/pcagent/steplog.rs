//! Read-only summary of a chat's PC steps: how many ran, by state.
use sqlx::SqlitePool;

/// Step counts for one chat, grouped by `pc_actions.state`, ordered by state name.
pub async fn summary(db: &SqlitePool, user_id: &str, chat_id: &str) -> sqlx::Result<Vec<(String, i64)>> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT state, COUNT(*) FROM pc_actions WHERE user_id = ? AND chat_id = ? GROUP BY state ORDER BY state",
    )
    .bind(user_id)
    .bind(chat_id)
    .fetch_all(db)
    .await
}

/// How many of a chat's latest steps the digest lists.
const DIGEST_STEPS: i64 = 10;
/// How much of each step's result the digest keeps.
const RESULT_CHARS: usize = 200;

/// The chat's latest steps, oldest first, one line each: time, state, summary and the start
/// of the result. None when the chat has no steps. Read-only.
pub async fn digest(db: &SqlitePool, user_id: &str, chat_id: &str) -> Option<String> {
    let mut rows = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT created_at, state, summary, result FROM pc_actions WHERE user_id = ? AND chat_id = ?
         ORDER BY created_at DESC, rowid DESC LIMIT ?",
    )
    .bind(user_id)
    .bind(chat_id)
    .bind(DIGEST_STEPS)
    .fetch_all(db)
    .await
    .ok()?;
    if rows.is_empty() {
        return None;
    }
    rows.reverse();
    let mut out = String::from("Steps already taken in this chat (oldest first):");
    for (at, state, summary, result) in rows {
        out.push_str(&format!("\n- {at} [{state}] {summary}"));
        if let Some(r) = result.filter(|r| !r.trim().is_empty()) {
            let flat = r.split_whitespace().collect::<Vec<_>>().join(" ");
            let cut: String = flat.chars().take(RESULT_CHARS).collect();
            let dots = if flat.chars().count() > RESULT_CHARS { "…" } else { "" };
            out.push_str(&format!(" → {cut}{dots}"));
        }
    }
    // FACTS: token-looking strings in commands or output never go back to a model.
    Some(crate::secrets::mask(&out))
}

/// FACTS: the tool that reads further back in the chat's step log than the digest.
pub const READ: &str = "read_command_history";
/// Steps one read returns at most (default 20), and characters of one step's result.
const READ_MAX: i64 = 50;
const READ_DEFAULT: i64 = 20;
const STEP_RESULT_CHARS: usize = 2_000;

pub fn spec() -> serde_json::Value {
    serde_json::json!({ "type": "function", "function": {
        "name": READ,
        "description": "Read the tool steps already run in this chat (newest page first, oldest first within it): the tool, its arguments, state and the start of its output. Use it to see what was done before, instead of running it again. Answered by Kompanion itself, not a computer.",
        "parameters": { "type": "object", "properties": {
            "before": { "type": "string", "description": "A step id: read the steps before it (the next, older page)." },
            "limit": { "type": "integer", "description": "Steps to read, 1 to 50 (default 20)." }
        }, "required": [] }
    } })
}

/// One page of the chat's steps, within `budget` characters, masked, saying when it was cut.
pub async fn read(db: &SqlitePool, user_id: &str, chat_id: &str, args: &serde_json::Value, budget: usize) -> String {
    let limit = args["limit"].as_i64().unwrap_or(READ_DEFAULT).clamp(1, READ_MAX);
    let cursor: Option<(String, i64)> = match args["before"].as_str().map(str::trim).filter(|b| !b.is_empty()) {
        Some(b) => match sqlx::query_as("SELECT created_at, rowid FROM pc_actions WHERE id = ? AND chat_id = ? AND user_id = ?")
            .bind(b).bind(chat_id).bind(user_id).fetch_optional(db).await
        {
            Ok(Some(c)) => Some(c),
            Ok(None) => return format!("error: there is no step {b} in this chat"),
            Err(e) => return format!("error: {e}"),
        },
        None => None,
    };
    let paged = cursor.is_some();
    let (at, rowid) = cursor.unwrap_or_else(|| ("\u{10ffff}".into(), i64::MAX));
    let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>)>(
        "SELECT id, created_at, state, tool, result FROM pc_actions WHERE user_id = ?1 AND chat_id = ?2
         AND (created_at < ?3 OR (created_at = ?3 AND rowid < ?4)) ORDER BY created_at DESC, rowid DESC LIMIT ?5",
    )
    .bind(user_id).bind(chat_id).bind(&at).bind(rowid).bind(limit + 1)
    .fetch_all(db)
    .await;
    let mut rows = match rows {
        Ok(r) => r,
        Err(e) => return format!("error: {e}"),
    };
    let mut more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    if rows.is_empty() {
        return if paged { "No older steps in this chat.".into() } else { "No steps have run in this chat yet.".into() };
    }
    let mut left = budget.clamp(1_000, 40_000);
    let mut page: Vec<String> = Vec::new();
    let mut oldest = String::new();
    for (id, at, state, tool, result) in &rows {
        let mut r = result.clone().unwrap_or_default();
        let n = r.chars().count();
        if n > STEP_RESULT_CHARS {
            r = format!("{}… [cut: {} more characters]", r.chars().take(STEP_RESULT_CHARS).collect::<String>(), n - STEP_RESULT_CHARS);
        }
        let entry = crate::secrets::mask(&format!("[{} {state}, id {id}] {tool}\n{r}", at.get(..16).unwrap_or(at).replace('T', " ")));
        if !page.is_empty() && entry.len() > left {
            more = true;
            break;
        }
        left = left.saturating_sub(entry.len());
        page.push(entry);
        oldest = id.clone();
    }
    page.reverse();
    let mut out = format!("{} step{} of this chat, oldest first:\n\n{}", page.len(), if page.len() == 1 { "" } else { "s" }, page.join("\n\n"));
    if more {
        out.push_str(&format!("\n\nOlder steps are left: read_command_history with before {oldest}."));
    } else {
        out.push_str("\n\nThese are all the steps of this chat.");
    }
    out
}

#[cfg(test)]
mod tests {
    /// Counts are grouped per state and scoped to the user and chat.
    #[tokio::test]
    async fn summary_counts_by_state() {
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        for q in [
            "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u', 'kees', '-', '2026-10-04')",
            "INSERT INTO machines (id, user_id, name, token_hash, created_at) VALUES ('m', 'u', 'soucouyant', 'h', '2026-10-04')",
            "INSERT INTO chats (id, title, updated_at) VALUES ('c', 'Build', '2026-10-04')",
            "INSERT INTO chats (id, title, updated_at) VALUES ('other', 'Other', '2026-10-04')",
            "INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, created_at) VALUES ('a1', 'c', 'u', 'm', '{}', 'one', 'done', '2026-10-04T10:00:00Z')",
            "INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, created_at) VALUES ('a2', 'c', 'u', 'm', '{}', 'two', 'done', '2026-10-04T10:01:00Z')",
            "INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, created_at) VALUES ('a3', 'c', 'u', 'm', '{}', 'three', 'failed', '2026-10-04T10:02:00Z')",
            "INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, created_at) VALUES ('a4', 'other', 'u', 'm', '{}', 'four', 'done', '2026-10-04T10:03:00Z')",
        ] {
            sqlx::query(q).execute(&db).await.unwrap();
        }
        let got = super::summary(&db, "u", "c").await.unwrap();
        assert_eq!(got, vec![("done".to_string(), 2), ("failed".to_string(), 1)]);
    }

    /// The digest is None for an empty chat, cuts long results at 200 characters,
    /// and leaves other chats out.
    #[tokio::test]
    async fn digest_truncates_and_scopes() {
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        for q in [
            "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u', 'kees', '-', '2026-10-04')",
            "INSERT INTO machines (id, user_id, name, token_hash, created_at) VALUES ('m', 'u', 'soucouyant', 'h', '2026-10-04')",
            "INSERT INTO chats (id, title, updated_at) VALUES ('c', 'Build', '2026-10-04')",
            "INSERT INTO chats (id, title, updated_at) VALUES ('empty', 'Empty', '2026-10-04')",
            "INSERT INTO chats (id, title, updated_at) VALUES ('other', 'Other', '2026-10-04')",
        ] {
            sqlx::query(q).execute(&db).await.unwrap();
        }
        assert!(super::digest(&db, "u", "empty").await.is_none());

        let long = "x".repeat(500);
        sqlx::query("INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, result, created_at) VALUES ('a1', 'c', 'u', 'm', '{}', 'long step', 'done', ?, '2026-10-04T10:00:00Z')")
            .bind(&long).execute(&db).await.unwrap();
        sqlx::query("INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, result, created_at) VALUES ('a2', 'c', 'u', 'm', '{}', 'short step', 'failed', 'boom', '2026-10-04T10:01:00Z')")
            .execute(&db).await.unwrap();
        sqlx::query("INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, result, created_at) VALUES ('a3', 'other', 'u', 'm', '{}', 'foreign step', 'done', 'nope', '2026-10-04T10:02:00Z')")
            .execute(&db).await.unwrap();

        let d = super::digest(&db, "u", "c").await.unwrap();
        assert!(d.contains(&format!("{}…", "x".repeat(200))), "long result is cut with an ellipsis");
        assert!(!d.contains(&"x".repeat(201)), "nothing past 200 characters");
        assert!(d.contains("[failed] short step → boom"));
        assert!(!d.contains("foreign step"), "other chats are left out");
        assert!(d.find("long step").unwrap() < d.find("short step").unwrap(), "oldest first");
    }

    /// FACTS: read_command_history pages back with a cursor, masks tokens, cuts long output and
    /// never shows another user's or chat's steps.
    #[tokio::test]
    async fn read_pages_masks_and_scopes() {
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        for q in [
            "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u', 'kees', '-', '2026-10-04'), ('x', 'ana', '-', '2026')",
            "INSERT INTO machines (id, user_id, name, token_hash, created_at) VALUES ('m', 'u', 'soucouyant', 'h', '2026-10-04')",
            "INSERT INTO chats (id, title, updated_at) VALUES ('c', 'Build', '2026-10-04'), ('other', 'Other', '2026')",
        ] {
            sqlx::query(q).execute(&db).await.unwrap();
        }
        for i in 0..5 {
            sqlx::query("INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, result, created_at) VALUES (?, 'c', 'u', 'm', ?, 's', 'done', ?, ?)")
                .bind(format!("a{i}"))
                .bind(format!(r#"{{"tool":"shell","command":"step {i}"}}"#))
                .bind(if i == 4 { format!("export GH={}\nexit: 0", crate::secrets::tests::fake("ghp_")) } else { "y".repeat(3_000) })
                .bind(format!("2026-10-04T10:0{i}:00Z"))
                .execute(&db).await.unwrap();
        }
        sqlx::query("INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, created_at) VALUES ('z', 'other', 'u', 'm', '{}', 'f', 'done', '2026-10-04T11:00:00Z')").execute(&db).await.unwrap();
        let args = serde_json::json!({ "limit": 2 });
        let r = super::read(&db, "u", "c", &args, 40_000).await;
        assert!(r.starts_with("2 steps of this chat, oldest first"), "{r}");
        assert!(r.find("step 3").unwrap() < r.find("step 4").unwrap(), "{r}");
        assert!(r.contains("GH=[hidden secret]") && !r.contains("Ab3dEf7h"), "{r}");
        assert!(r.contains("[cut: 1000 more characters]"), "{r}");
        assert!(r.ends_with("read_command_history with before a3."), "{r}");
        let r = super::read(&db, "u", "c", &serde_json::json!({ "before": "a1" }), 40_000).await;
        assert!(r.contains("step 0") && !r.contains("step 1") && r.ends_with("These are all the steps of this chat."), "{r}");
        assert_eq!(super::read(&db, "u", "c", &serde_json::json!({ "before": "a0" }), 40_000).await, "No older steps in this chat.");
        assert!(super::read(&db, "x", "c", &serde_json::json!({}), 40_000).await.starts_with("No steps"), "another user's steps stay hidden");
        assert!(super::read(&db, "u", "c", &serde_json::json!({ "before": "z" }), 40_000).await.starts_with("error:"), "a step of another chat is no cursor");
        // A small budget keeps the first step and says more are left.
        let r = super::read(&db, "u", "c", &serde_json::json!({}), 1_000).await;
        assert!(r.starts_with("1 step of this chat") && r.contains("Older steps are left"), "{r}");
    }

    #[tokio::test]
    async fn digest_masks_tokens() {
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        for q in [
            "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u', 'kees', '-', '2026-10-04')",
            "INSERT INTO machines (id, user_id, name, token_hash, created_at) VALUES ('m', 'u', 'soucouyant', 'h', '2026-10-04')",
            "INSERT INTO chats (id, title, updated_at) VALUES ('c', 'Build', '2026-10-04')",
        ] {
            sqlx::query(q).execute(&db).await.unwrap();
        }
        sqlx::query("INSERT INTO pc_actions (id, chat_id, user_id, machine_id, tool, summary, state, result, created_at) VALUES ('a', 'c', 'u', 'm', '{}', ?, 'done', 'ok', '2026-10-04T10:00:00Z')")
            .bind(format!("curl with {}", crate::secrets::tests::fake("ghp_"))).execute(&db).await.unwrap();
        let d = super::digest(&db, "u", "c").await.unwrap();
        assert!(d.contains("curl with [hidden secret]") && !d.contains("Ab3dEf7h"), "{d}");
    }
}
