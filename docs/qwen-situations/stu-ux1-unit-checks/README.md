# STU-UX1 unit checks (Kreative Studio)

The checks and tools every STU-UX1 situation reruns with, gzipped as text so linters and PR-Agent skip them.

- `ux1.e2e.js`: the browser test (copy to the Studio repo's `test/e2e/`; `UX1=<sections>` picks sections).
- `units.txt`: unit number and its check command; `node.sh`/`e2e.sh`/`helpers.js`/`nolightbox.js`: the checks.
- `stages.py`: builds the tree before and after each unit from Claude's reference version (every check was
  run on both: fail before, pass after).
- `mkjobs.py`: the 12 job prompts; `runux.sh`: one pipeline run per unit on kireserver.
