# 277: STU-UX2 (Coder OVMS, unit 4 forms.js, 3 attempts)

set the error text only when creating the element (a second call did not update it), inverted the char-counter branch (inserted after the input instead of after the counter), cleared only INPUT elements (not textarea), and added an unrequested validateForm(). Claude fixed 4 spots.

Rerun: put `input-forms.js.txt` back as `public/js/forms.js` in a Studio tree at the stage before unit 4 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u4-forms*.
