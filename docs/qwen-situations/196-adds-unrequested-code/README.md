# Lesson 196: the Coder adds code the prompt said not to write

Situation: GATE-IN-CI (2026-10-12), job `ru1` (jobs-all.json, OVMS): create tools/review_status.py with ONE function `status_for(code)`; the prompt said "do not write sweep or anything else yet".
Wrong output (attempt 1, check passed): 82 lines: status_for plus a copy of the test file's fake `Api` class, a `sweep` that posts an "error" status on any exception, a `StatusClient`, and a placeholder `main` ("fetch PRs here"). The check only exercised status_for, so it passed.
Corrected: Claude cut the file to the docstring and status_for (9 lines) and wrote `sweep` itself later (lesson 197).
Lesson: a "do not write X" sentence is not enforced; a check for a new file must also fail on extra top-level names (e.g. assert the module's public names equal the expected set). Rerun after model refinement: run jobs-all.json, job ru1.
