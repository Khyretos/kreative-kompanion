# Lesson 231: sqlx calls inside a fn returning Result<Value, String> (2026-10-13, PROJ-OVERVIEW o6 o10 o11 o12)

Four async database units were hard-stuck after three attempts each, with the same family of compiler errors the repair rounds never fixed: a bare `?` on a sqlx::Error in a function that returns Result<_, String>; `fetch_all(s.db)` instead of `&s.db`; a missing `.await`; `query_as::<_, TaskRow>` on a struct without FromRow; `if let Some(x) = &p.repo_git` on a plain String; a TEXT id read as i64; `Settings::parse` treated as a Result. o10 also invented `s.host.online()` and awaited a spawned task. o6 (pure) counted total without queued tasks and listed the goal's own index as its tasks.

Claude rewrote the four bodies (corrected.txt). From o13 on the prompt gave the exact chain `.fetch_all(db).await.map_err(|e| e.to_string())?` and "query into tuples": o13, o14 and o15 then passed first try.

Rule: for async DB code give the full call chain as a code line, the tuple type of every query, and the type of each field the code touches (String vs Option<String>).
