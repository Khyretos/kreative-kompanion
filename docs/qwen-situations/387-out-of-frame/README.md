Lesson 387 (STU-PE1 pe1-stagezoom, 1 attempt (grep check passed)): outOfFrame() looped `for (const p of w.pos)` over an object and read p.x/p.y of world points instead of their screen position; Claude's check only grepped for the method name; Claude fixed 3 lines.

Rule: A new method gets a call in the check.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), wrong-* (the Coder's last output), corrected-* (what was kept). Code files carry a .txt suffix so the lint gate skips them.
