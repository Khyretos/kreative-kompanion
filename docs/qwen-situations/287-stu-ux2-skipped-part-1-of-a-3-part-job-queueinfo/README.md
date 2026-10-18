# 287: STU-UX2 (Coder OVMS, unit 12 cardInner queue/fail, 3 attempts)

skipped part 1 of a 3-part job (queueInfoHtml in the thumbnail) and its repair rounds invented elsewhere instead: a getElapsed() helper nobody calls, a 'Queued' div appended in updateJob (crashes for a new job, card is null) and a renamed DOM lookup. Claude reverted those and added the one call. Also found: main.js's escapeHtml did not escape quotes, so any error text with " broke its title attribute (fixed through the Coder as unit 20, first try).

Rerun: put `input-main.js.txt` back as `public/js/main.js` in a Studio tree at the stage before unit 12 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u12-queuecard*.
