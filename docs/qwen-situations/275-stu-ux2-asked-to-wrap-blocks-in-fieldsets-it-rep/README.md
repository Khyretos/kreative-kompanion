# 275: STU-UX2 (Coder OVMS, unit 1 run 2, 3 attempts failed)

asked to WRAP blocks in fieldsets, it REPLACED the wrapped div (class preset-group gone), dropped `<div id="sheet-extras">` (page error, JS null), dropped a comment and changed an unrelated `</section>` far away. Claude wrote it by script. Prompt rule: for a wrap, give the exact first and last line of each block and say "keep both lines, add the fieldset lines around them".

Rerun: put `input-index.html.txt` back as `public/index.html` in a Studio tree at the stage before unit 1 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u1-groups*.
