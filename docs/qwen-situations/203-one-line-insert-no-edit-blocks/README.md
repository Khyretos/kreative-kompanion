# Lesson 203: a one-line insert after a given line produced no edit blocks

Situation: RUNNER-SYNC unit rsw3 (rsw3.json): add one `updateRunner(...)` method line after the `unpairMachine` line in web/src/api/http.ts (a 300-line file). The Coder (soucouyant, 3 attempts) first answered without SEARCH/REPLACE blocks twice, then the repair rounds rewrote the unrelated `transcribe` method (wrong-output.diff) and ran out of tokens.
Fix: Claude added the one line.
Rule: for a one-line insertion into a long file, quote the exact anchor line in the prompt and put `focus` on that line only; a retry that fed back "no edit blocks" should repeat the block format. Candidate pipeline fix: after "no edit blocks found", the repair prompt must restate the block format.
