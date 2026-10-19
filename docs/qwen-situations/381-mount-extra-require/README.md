Lesson 381 (STU-PE1 pe1-mount, 1 attempt (check blocked by the api unit)): Asked for one inserted line, it also added an unused require of skeleton.js at the top; Claude removed it.

Rule: Checks for "change nothing else" patches count the changed lines.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), wrong-* (the Coder's last output), corrected-* (what was kept). Code files carry a .txt suffix so the lint gate skips them.
