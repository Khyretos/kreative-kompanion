Lesson 371 (STU-PE1 pe1-vec, 3 attempts): signedAngle took the sine from len(cross), so the angle was never negative although the prompt said atan2(dot(axis, cross), dot); Claude fixed one line.

Rule: When a sign matters, the test has a negative case and the prompt says where the sign comes from.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), `wrong-*` (the Coder's last output), `corrected-*` (what was kept). Code files carry a .txt suffix so the lint gate skips them.
