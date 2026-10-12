# Lesson 206: weak grep checks let a wrong-shaped edit pass (two units)

Situation: RUNNER-SYNC units rsw4 and rsw5 (rsw45.json, soucouyant, passed in 2 and 3 attempts).

- rsw5 was asked to REPLACE a span chip by one button. The Coder kept the span (only capitalising its text to "Update to") and added a second, icon-only button with the right `data-action`. The check only grepped for the attribute, the text and the absence of the lowercase old text.
- rsw4 was asked to add `async updateRunner() {}` directly after `async unpairMachine() {}` (line 391); the Coder put it at line 627, after an unrelated method. The check only grepped the line.
  wrong-output.diff.gz has both. Fix: Claude applied the intended edits.
  Rule (family of 25): a replace job's check must fail on the old element (grep for its class/title) and count the new one; an insert-after job's check must compare the line number or the preceding line (`grep -B1`).
