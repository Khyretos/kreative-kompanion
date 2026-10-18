# 279: STU-UX2 (Coder soucouyant, unit 7 toast.js, passed at attempt 2)

dropped the spec'd default `timeout = 5000` (used opts.timeout directly, so a toast without a timeout never left). The check passed because every toastapi call gave a timeout: Claude added the default; checks should also cover defaults.

Rerun: put `input-toast.js.txt` back as `public/js/toast.js` in a Studio tree at the stage before unit 7 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u7-toast*.
