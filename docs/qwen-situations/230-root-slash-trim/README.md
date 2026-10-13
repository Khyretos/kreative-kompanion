# Lesson 230: the root folder "/" after trim_end_matches (2026-10-13, PROJ-OVERVIEW o3)

Job: `covers(target, folder)` in server/src/overview.rs (a computer grant reaches the repo folder). The prompt said "a bare / stays /". The Coder (OVMS) trimmed trailing slashes with `trim_end_matches('/')`, which turns "/" into "", then rejected the empty target, so the root grant never covered anything. Six attempts (two runs of three; in the first run my check hid the assert text) kept the trim before the root check. Run 1 also added a reverse case (a grant inside the repo covers it) and `folder == "/" -> true`.

Claude corrected it: a trim helper that keeps "/" (corrected.txt).

Rule: give edge cases as a table of input -> output in the prompt and name the trap ("trim_end_matches turns / into an empty string"). Files: o3-job.json, run1-wrong.rs.gz, run2-wrong.rs.gz, corrected.txt.
