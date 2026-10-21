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
    Some(out)
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
}
