---
extends: worker/rust/SKILL
---
4. (2026-10-03) Use the helpers that exist instead of re-reading tables: `admin::load(&db)` for settings, `util::now()` for timestamps, `auth::create_session*` for sessions.
8. (2026-10-03) Errors shown to people: never pass a raw response body through. Use `llm::readable_error` (no HTML, at most 200 characters, a plain word for known GPU failures).
36. The drafting pipeline (`tools/qwen/pipeline.py`) strips code fences from whole-file answers, so
    a file with a ``` inside a string or doc comment comes back cut to a fragment. Use
    `"mode": "patch"` (search/replace blocks) for such files.
115. (2026-10-08) The prompt must say "do not add a tests module or tests": a job whose file already ends in `#[path = "x_tests.rs"] mod tests;` got a second inline `mod tests` (E0428) and a scanner with `Lines::peek` and an out-of-scope `j`, 3 fix rounds and still red; a string scanner of 30 lines is cheaper for Claude to write after the first failed draft (CHAT-05).
