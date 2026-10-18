# 282: STU-UX2 (Claude, job prompt + Coder soucouyant, unit 16 ux2-forms.css, 3 attempts)

the job did not say that style.css's `textarea:focus { border-color: transparent }` outranks `[aria-invalid="true"]`, and fieldError focuses the field, so the red border never showed; the Coder also added an unrequested `.invalid-field` copy and wrote `animation: x var(--motion) ease-out` although --motion already holds the curve (invalid shorthand, no animation). Job prompts for CSS must name the existing rules that compete; Claude fixed 3 spots.

Rerun: put `input-ux2-forms.css.txt` back as `public/css/ux2-forms.css` in a Studio tree at the stage before unit 16 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u16-formcss*.
