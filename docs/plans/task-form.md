# TASK-FORM: the New task form (2026-10-12)

Before: Description was one monospace textarea prefilled with the literal template `**Goal:** / **Steps** / 1. / **Done when:**` that had to be edited around; Depends on was a bordered box of checkboxes that broke its layout on a phone (checkbox and title in separate columns); the Kind, Part of and State selects used the native control and its grey drop-down list.

After: Goal, Steps (one per line, numbered for you) and Done when are separate fields, and a closed "More (Markdown)" drawer holds anything else; on save they compose the same markdown the server already checks, and editing a task splits its description back into the fields (text that does not fit the shape goes to the drawer, which then opens). Depends on is a search box with chips: type to filter, click or Enter to add, the chip's own button removes it; a task never offers itself. The selects have no native look and their option list uses the purple input colour. Nothing reloads: the picker only toggles attributes on elements rendered once.

Files: web/src/views/taskdesc.ts (composeDescription, parseDescription), web/src/views/deppicker.ts (depPicker, filterDeps, setDep), tasks.ts editor, tasktree.ts treeFields, main.ts wiring, styles.css. Tests: web/tests/taskform.spec.ts (5 tests, incl. 390 px light and dark, selects match the text fields, no sideways overflow); tasktree.spec.ts uses the new fields.

Checks: tsc clean; full Playwright 260 of 261 passed on kireserver (the one failure, overseer.spec.ts OVR-01b, passes 3 of 3 alone and does not touch the task form).

## Coder run log (OVMS on kireserver)

| Unit | File | Drafts | Attempts | Result |
| --- | --- | --- | --- | --- |
| tf1 | taskdesc.ts (new) | 1 | 1 | passed Claude's first check, review found 4 bugs |
| tf1r | taskdesc.ts repair | 2 | 2 | passed the stronger check; Claude still replaced parseDescription (lesson 216) |
| tf2 | deppicker.ts depPicker (new) | 1 | 1 | Coder, kept unchanged |
| tf3 | deppicker.ts filterDeps, setDep | 3 | 3 | failed; Claude fixed the count in filterDeps (lesson 217) |
| tf4 | tasktree.ts treeFields | 1 | 1 | Coder, kept unchanged |
| tf5 | tasks.ts editor fields | 3 | 3 | failed; Claude applied the edit by script (lesson 218) |
| tf6a to tf6e | main.ts, one edit per job | 5 | 1 each | Coder, kept unchanged |
| tf7 | styles.css | 1 | 1 | Coder, kept unchanged |

Lines added in web/src: 191; written by Claude: 40 (parseDescription 18, filterDeps 5, editor fields 17); Coder lines kept: 151 (79 %). GPU 314.9 s, 133,005 prompt and 8,541 output tokens over 17 drafts. Logs: docs/qwen-situations/task-form-unit-runs/; totals: docs/qwen-log/task-form.jsonl.

Splitting main.ts into one edit per job (after lesson 218) took the Coder from 3 failed attempts on a multi-edit job to 5 of 5 first-try passes.

Lessons: 214 (deploy from a dirty checkout), 215 (precheck skipped new-file units), 216, 217, 218.
