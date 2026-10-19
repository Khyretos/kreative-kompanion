Lesson 382 (STU-PE1 pe1-cli, 3 attempts): It copied the broken loader from lib/pose-api.js (given as context before Claude fixed it), imported a checkPose that does not exist and parsed the --ops path as JSON; Claude fixed 8 lines.

Rule: Never pass a context file that has not passed its own check yet; the Coder copies its mistakes.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), `wrong-*` (the Coder's last output), `corrected-*` (what was kept). Code files carry a .txt suffix so the lint gate skips them.
