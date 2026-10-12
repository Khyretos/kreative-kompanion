# Lesson 220: "keep the first date" became a read-then-write with invented types, and a kept line was dropped

Situation: PROJ-ARCHIVE unit s3b, patch server/src/projects.rs (PATCH /api/projects/{id}/settings), OVMS, two fresh runs of 3 attempts each, all failed. Job: s3b.json; the file before the job: s3b-before.txt.gz. The prompt asked for: Some(true) sets `archived_at` to `util::now()` but keeps the first date (it named `COALESCE(archived_at, ?)`), Some(false) sets NULL, and said the `s.bus.send(...)` line after the block stays.

What the Coder did:

- run 1 (s3b-run1-wrong.diff.gz): built the SQL with `format!`, swapped true and false (archive set NULL), passed the timestamp to a format string without a placeholder.

- run 2 (s3b-wrong.diff.gz): a SELECT COALESCE first, read as `Option<i64>` (timestamps are strings), then a second UPDATE; the repair rounds kept the shape and deleted the `s.bus.send(... "projects" ...)` line it was told to keep.

Fix: Claude wrote the two one-line UPDATEs (s3b-corrected.diff.gz): `SET archived_at = COALESCE(archived_at, ?)` bound to `util::now()`, and `SET archived_at = NULL`.

Rule: for a "keep the existing value" update, give the finished SQL statement in the prompt, not the idea; say the column type (TEXT timestamp from util::now()). A line that must survive belongs in the check too (the test checks the Changed event now), because "X stays as it is" alone does not hold through repair rounds.
