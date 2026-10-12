# GATE-IN-CI: the review gate as a required CI status

Before: the review gate (`tools/qwen/review_findings.py`) only ran inside auto-merge, so a PR looked mergeable (all CI green) while PR-Agent findings were still open.
After: the workflow `review-gate` runs the gate for every open PR on each PR event, comment and every 10 minutes, and posts the commit status `review-gate / gate` (success / pending / failure / error). The status is in `required` of `.forgejo/auto-merge.json`, so auto-merge waits for it.

Files: `tools/review_status.py` (status_for, sweep), `tools/review_status_io.py` (StatusClient, main), `tools/test_review_status.py`, `.forgejo/workflows/review-gate.yml`, `.forgejo/auto-merge.json`.

Needs the secret AUTO_MERGE_TOKEN (already used by auto-merge); without it the job posts nothing. Branch protection ("required status checks") is a repo setting and not changed here: add `review-gate / gate` there to hide the merge button too.
This PR itself has no `review-gate / gate` status (the workflow runs from main): merge it by hand once the other checks and the gate are green.

## Per-unit run log (loop: one draft, up to 3 repair attempts, OVMS and soucouyant in parallel)

| unit | model | attempts | result | lines drafted / kept | lesson |
|---|---|---|---|---|---|
| ru1 status_for | OVMS Coder | 1 | passed, but 82 lines instead of 9 | 82 / 9 | 196 |
| ru2 sweep | OVMS Coder | 4 (1 + 3 repairs) | failed, invented API names | 15 / 0 | 197 |
| ru3 StatusClient | soucouyant qwen3.5 9B | 1 | passed | 52 / 52 | none |
| ru4 main | soucouyant qwen3.5 9B | 1 | passed, one wrong index | 24 / 23 | 198 |
| ru5 review-gate.yml | OVMS Coder | 1 | passed | 40 / 40 | none |
| ru6 auto-merge.json | soucouyant qwen3.5 9B | 1 | passed | 1 / 1 | none |

Local models: OVMS 6 calls, 132.5 GPU s, 30,520 prompt / 3,835 output tokens; soucouyant 3 calls, 36.8 GPU s, 20,295 prompt / 1,523 output tokens (docs/qwen-log/gate-in-ci-*.jsonl). Claude tokens: unmeasured (no per-task counter).
Tests (written first, failing on the untouched tree): tools/test_review_status.py, 11 tests; black and isort applied.
