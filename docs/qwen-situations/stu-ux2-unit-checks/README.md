# STU-UX2 unit checks (Kreative Studio)

The checks and tools every STU-UX2 situation reruns with, gzipped as text so linters and PR-Agent skip them.

- `ux2.e2e.js`: the browser test (copy to the Studio repo's `test/e2e/`; `UX2=<sections>` picks sections); `lighthouse.e2e.js` and `ux2-shots.e2e.js` likewise.
- `units.txt`: unit, tree (o = OVMS, s = soucouyant) and check command; `chk-*`: the checks.
- `mkjobs.py`: the job prompts; `runux2.sh`: one pipeline run per unit on kireserver inside kk-sandbox.
