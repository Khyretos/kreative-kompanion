# Lesson 224: a timer cleared a shared "current toast" variable, and the error path restored a stale list

Situation: PROJ-ARCHIVE unit w5e, patch web/src/main.ts (new fn archiveProject), OVMS, passed its check on the first attempt; Claude corrected small leftovers on review. Job: w5e.json; before: w5e-before.txt.gz.

What the Coder did (w5e-wrong.diff.gz):

- the 6 s timer ran `archiveToast?.remove()`: archiving A and then B within 6 s removed B's toast when A's timer fired;

- on a server error it restored `before.map(...)`, the project list from before the call, which drops any change that arrived meanwhile;

- it dropped the two-line header comment it was given and placed the function above changeChat instead of where asked.

Fix (w5e-corrected.diff.gz): the timer removes its own toast and clears the variable only when it still points at it; the error path maps over `store.get().projects`; the comment is back.

Rule: tsc and the happy-path test cannot see these; review timers and rollbacks of optimistic updates by hand, and put the "two quick archives" case into the Playwright spec when the toast matters.
