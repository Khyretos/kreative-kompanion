# Lesson 219: the check printed only the error headers, so the repair rounds were blind

Situation: PROJ-ARCHIVE unit s3b (server/src/projects.rs, fn settings, OVMS), first run. Job: s3b.json. The check (`sh /tmp/kk-arch-chk/s3b.sh`, cargo test) filtered the compiler output through `grep -E '^error|-->|...'`, so each repair round got `error: argument never used` and a list of `-->` locations, but not the code lines or the help text under them.

What happened: both repair rounds changed one line and kept the wrong `format!("SET archived_at = ?", timestamp)`; the Coder never saw which argument was meant. This was Claude's check fault, not a Coder mistake, so the unit was run again with the fixed check (run 2 is lesson 220).

Fix: the check prints whole compiler error blocks (from a line starting `error` to the next `warning`), warnings left out, then the test failure lines. common-fixed.sh.txt.gz is the fixed helper.

Rule: a check feeds the repair round. Print the complete rustc/tsc error block (code line, caret, help), never only the header lines; strip warnings, not context.
