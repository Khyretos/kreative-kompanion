# Lesson 239: one column read as a tuple (2026-10-13, PROJECT-SHARING sharing.rs)

The Coder drafted the whole 150-line module right (access, members, add, remove) and failed one line three times: `let user_id: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE name = ?")`, then `.bind(&user_id)` and `user_id == u.id` on the tuple (E0277, E0308). Both fix rounds got the same compile errors and changed other lines. Claude changed that one line to `Option<String>` with `query_scalar` (see `corrected.rs`).

Rule: for a single column the prompt says `sqlx::query_scalar` and the type `Option<String>`; the same lesson as 184 for `Option`. A check that repeats the same error after a fix round should name the line to the fix round.

Claude's own check fault in the same unit: `cargo test sharing:: -- name1 name2` runs the whole module because the first argument is a prefix filter, so the later wiring test failed the first unit; give names only after `--`.
