Lesson 377 (STU-PE1 pe1-ops, 3 + 3 repair attempts): A 450-line draft lost every note (fresh [] per store call), started set_joint from zeros, called .includes on an object, reassigned parameters in reset/preset and copied mirrors without clamping; Claude rewrote ops.js (130 lines) around one store(p, name, e, notes).

Rule: One op per unit (or a dispatch unit plus one unit per op); say "pass the same notes array everywhere".

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), wrong-* (the Coder's last output), corrected-* (what was kept). Code files carry a .txt suffix so the lint gate skips them.
