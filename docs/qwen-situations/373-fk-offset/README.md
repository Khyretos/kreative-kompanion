Lesson 373 (STU-PE1 pe1-fk, 3 attempts): The child offset used the solved parent position (rest(j) - pos[p]) instead of rest(j) - rest(p); Claude fixed one line.

Rule: Name both lookups (S.byName[j].rest - S.byName[parent].rest) and test with a moved root.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), wrong-* (the Coder's last output), corrected-* (what was kept). Code files carry a .txt suffix so the lint gate skips them.
