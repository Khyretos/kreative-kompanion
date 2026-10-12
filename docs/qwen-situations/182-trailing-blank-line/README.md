# Lesson 182: Coder cannot end a file with exactly one newline

Situation: jobs `gt-report-row` and `gt-plan-metrics` (GATE-TOOLS report, 2026-10-12): insert one table row into docs/ai-capability/report-2026-10-12.md; append a Metrics block to the end of docs/plans/gate-tools.md. Checks: check-row.py and check-plan.py (fail on the untouched tree, pass on the corrected one). job.json holds both jobs, job-repair.json the repair round that was fed the check error.
Wrong output: row job passed 6/6 (OVMS 3, soucouyant 3). The append job failed 6/6: OVMS 3/3 wrote the block correctly but left a second blank line at the end of the file; a repair round fed the check error (job-repair.json) changed nothing, 3/3. soucouyant run 1 dropped the heading and the first table lines, runs 2 and 3 left the extra blank line like OVMS.
Corrected: Claude removed the trailing blank line (`rstrip("\n") + "\n"`), nothing else; the corrected file is docs/plans/gate-tools.md in this PR.
Rerun after model refinement: run job.json (plan job) and job-repair.json with pipeline.py on both Coders from an untouched copy of main and see whether the file ends with exactly one newline.
Per-run records: runs/ (pipeline log lines; the full prompts are job.json and job-repair.json).
