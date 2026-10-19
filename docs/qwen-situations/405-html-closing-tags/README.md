# Lesson 405

Lesson 405 (STU-PE2 pe2-html, Coder): The patch replaced the Pose panel closing `</div></section>` with the new section, so every later tab sat inside the Pose tab (ux1/ux2 e2e broke), and put quat/anim/clips after depth-parts.js instead of depth.js. Claude restored the 2 tags and moved the 3 script lines.

Rule: An HTML insert job says which closing tags stay, and its check verifies tag balance and runs every existing e2e, not just an id count.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
