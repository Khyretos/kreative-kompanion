# Lesson 218: verbatim replacement fields were rewritten from the block they replaced

Situation: TASK-FORM unit tf5, patch web/src/views/tasks.ts (task editor), OVMS, 3 attempts, all failed. Job: tf5.json. The prompt gave three edits with the exact new lines: add `import { parseDescription } from "./taskdesc";`, delete `TEMPLATE`, add `const d = parseDescription(...)` and replace the one Description field with four guided fields (Goal, Steps, Done when, the Markdown drawer).

What the Coder did (same in every attempt, wrong-output.diff.gz is the final one): the import became an empty line, TEMPLATE became an empty line, `const d` was never added, and the new fields kept the old field's `name="description"` and `${t?.description ?? ""}` instead of `name="goal"` / `${d.goal}`; the Done when field was left out. The check message "textarea id=\"task-goal\" name=\"goal\" expected 1, received 0" did not change the answer.

Fix: Claude applied the edit as given (by script).

Rule: lesson 192/195 again, now for a template block: when the replacement is verbatim, the 9B blends it with the SEARCH block (copies old attribute values) and turns the new lines it cannot place into blank lines. Apply verbatim edits by script and give the Coder only the parts that need judgement, or split it into one job per edit with a SEARCH line that occurs once.
