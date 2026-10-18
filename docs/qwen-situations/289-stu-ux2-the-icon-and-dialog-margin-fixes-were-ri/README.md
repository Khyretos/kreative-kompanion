# 289: STU-UX2 (Coder OVMS, unit 25 CSS polish, 3 attempts)

the icon and dialog-margin fixes were right; for the narrow waveform it guessed: made the canvas (a grid item) a flex item with min-width 50 %, used flex-basis auto (the canvas width attribute, set from its client width, then keeps it small) and left a contradictory `.playing .i-pause { display: none }`. Claude fixed 3 rules. Job prompts for layout fixes should say which box is flex and which is grid.

Rerun: put `input-ux2.css.txt` back as `public/css/ux2.css` in a Studio tree at the stage before unit 25 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u25-polish*.
