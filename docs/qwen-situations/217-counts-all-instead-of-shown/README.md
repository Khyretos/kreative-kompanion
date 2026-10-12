# Lesson 217: "count the shown options" became "count all options", plus an invented fallback

Situation: TASK-FORM unit tf3, append filterDeps and setDep to web/src/views/deppicker.ts, OVMS, 3 attempts, all failed. Job: tf3.json. Check: tf3.sh runs zz-chk-tf3.spec.ts (Playwright on a bare page with the bundled module).

The Coder wrote `const shown = picker.querySelectorAll(".dep-option").length;` before the loop, so `.dep-none` ("No matching task") never showed when the search matched nothing. The check message "filterDeps with no match: .dep-none must be shown" came back three times; each repair added code instead of fixing the count: a branch that creates the `.dep-none` element with document.createElement when it is missing (the template always renders it) and a second `if (shown === 0 ...)` that can never be true. setDep was right in every attempt. wrong-output-deppicker.ts.gz.

Fix: Claude counted inside the loop (`if (!o.hidden) shown++`) and removed the invented fallback.

Rule: say "count only the options left visible, inside the loop" in the prompt, not "count the shown options". Candidate pipeline help: when the same check line repeats, show the Coder its own line that computes the value the message names.
