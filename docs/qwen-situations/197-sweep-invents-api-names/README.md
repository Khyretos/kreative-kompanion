# Lesson 197: the Coder invents API names although the prompt names them

Situation: GATE-IN-CI, job `ru2` (jobs.json, OVMS, patch mode): add `sweep(api, gate)` that loops `api.open_pulls()`, reads `pull['number']`, `pull['head']['sha']`, calls `api.post_status(sha, state, description)`.
Wrong output (4 attempts, all failed): `for pr in api.list_prs()`, `pr.number`, `pr.head`, `api.set_status(...)`, errors swallowed with `except Exception: pass`; the repair rounds only changed one line each. See runs/wrong-ovms-review_status.py.txt. Same family as lesson 189 (invents APIs instead of copying a template).
Corrected: Claude wrote the 15 lines.
Lesson: for a function that calls a fake API, put the fake class's source in "context" (here the test's `Api`) so the names are copied, not remembered; list the dict keys. Rerun after refinement: run jobs.json.
