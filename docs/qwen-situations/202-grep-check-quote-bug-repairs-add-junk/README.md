# Lesson 202: a check with a bad grep made good drafts look wrong, and the repairs added junk

Situation: RUNNER-SYNC unit rsw1 (rsw1.json). The first draft added `runnerVersion: "0.5.0", runnerLatest: "0.6.0",` correctly, but Claude's check script used `grep -q runnerLatest:."0.6.0"` which, after the shell removed the quotes, never matched. Two repair rounds (OVMS) then changed unrelated lines in web/src/api/mock.ts (an `as const`, a new rx9070 timeline entry; wrong-output.diff.gz). Same family as 200.
Fix: Claude reverted the file and reran with a fixed check (`grep -F`).
Rule: write the check pattern with `grep -F` and run the check by hand against a hand-made correct file before queueing, not only against the untouched tree.
