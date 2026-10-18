# 283: STU-UX2 (Coder OVMS, unit 6 queue.js, 3 attempts)

`^(\w+(?:Error|Exception)):` needs a word before "Error" so a bare "Error: " stayed; ordinal() indexed a 4-item suffix array with n % 10 ('4undefined'); invented wording ('Starting next' for position 1, '0th in line' for 0) instead of the spec's 'Next'/'Starting'; escapeHtml on a textContent value. Claude fixed 4 lines.

Rerun: put `input-queue.js.txt` back as `public/js/queue.js` in a Studio tree at the stage before unit 6 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u6-queue*.
