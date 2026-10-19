Lesson 378 (STU-PE1 pe1-depthparts, 3 attempts): at() read S.pos/S.rot/S.rest (they do not exist) instead of w.pos/w.rot/S.byName[j].rest, the palm became a chain between knuckles, and leg segments were looked up in a map that only had arms; Claude fixed 6 spots.

Rule: Give each lookup's object in a one-line helper verbatim.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), `wrong-*` (the Coder's last output), `corrected-*` (what was kept). Code files carry a .txt suffix so the lint gate skips them.
