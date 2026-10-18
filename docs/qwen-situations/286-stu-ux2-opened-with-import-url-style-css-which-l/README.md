# 286: STU-UX2 (Coder soucouyant, unit 17 ux2.css, 3 attempts)

opened with `@import url('style.css')`, which loads style.css again after ux1.css and silently undoes UX1's overrides; a repair round answered "AUDIO has no focus ring" with `audio:focus-visible { outline: none }` (the opposite) plus a duplicate block. Claude removed both. Also my check for unit 17 included the native audio element that unit 13 removes (same mistake as L280) and the contrast section filled a field on a hidden tab (test bug, fixed).

Rerun: put `input-ux2.css.txt` back as `public/css/ux2.css` in a Studio tree at the stage before unit 17 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u17-ux2css*.
