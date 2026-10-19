Lesson 385 (STU-PE1 pe1-library, 1 draft + 1 review repair): An undefined `stage`; the repair turned it into `need.stage`, which passed Claude's grep check; cards had no data-id; Claude fixed it.

Rule: Checks test behaviour, never the absence of a string.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), `wrong-*` (the Coder's last output), `corrected-*` (what was kept). Code files carry a .txt suffix so the lint gate skips them.
