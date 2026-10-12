# Lesson 223: the SEARCH block took the line above along, and the repairs invented a state field instead

Situation: PROJ-ARCHIVE unit w6, patch web/src/views/tasks.ts (renderTasks "All" scope), OVMS, 3 attempts failed. Job: w6.json; before: w6-before.txt.gz. Asked: replace the one `const list = ...` line with a comment, `const listed = ...` and the new `const list`.

What the Coder did (w6-wrong.diff.gz): its block replaced `const scope = ...` (the line above) as well, so `scope` no longer existed. The repair rounds, given "Cannot find name 'scope'", wrote `s.scope` everywhere (AppState has no such field) and wrapped the list in `s.rightTab === "tasks" && (...)`.

Fix: Claude applied the requested replacement to the original file (w6-corrected.diff.gz).

Rule: when the line to replace sits right under a declaration the rest of the function uses, quote the line above in the prompt as "keep this line exactly"; a repair round that says "Cannot find name X" needs the hint "X was declared in the line you removed".
