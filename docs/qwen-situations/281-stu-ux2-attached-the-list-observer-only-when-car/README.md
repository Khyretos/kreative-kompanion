# 281: STU-UX2 (Coder soucouyant, unit 9 keys.js, 3 attempts)

attached the list observer only when cards existed at load (they come later), and tested an open dialog with the non-existent `document.dialogs`. Claude fixed 2 lines.

Rerun: put `input-keys.js.txt` back as `public/js/keys.js` in a Studio tree at the stage before unit 9 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u9-keys*.
