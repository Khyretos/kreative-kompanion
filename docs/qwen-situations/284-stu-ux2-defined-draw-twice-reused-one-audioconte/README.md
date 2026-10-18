# 284: STU-UX2 (Coder OVMS, unit 8 players.js, 3 attempts)

defined draw() twice, reused one AudioContext after closing it (second decode fails), the observer only drew a node that IS a .wave-player (not players inside added cards) and there was no first pass, a click anywhere in the player toggled play (so seeking also played), the timeupdate handler got the event instead of the player, and bars were spaced in CSS px on a DPR-scaled canvas. Claude rewrote most of it (64 of its lines replaced).

Rerun: put `input-players.js.txt` back as `public/js/players.js` in a Studio tree at the stage before unit 8 (the other files as in Studio commit ca03535's parent plus the earlier units), run `job.json` with tools/qwen/pipeline.py; `wrong-*` is the last attempt, `corrected-*` the file Claude kept. The check command is in job.json (tools in ../stu-ux2-unit-checks/). Traces: kireserver /tmp/kk-ux2/trace/ux2-u8-players*.
