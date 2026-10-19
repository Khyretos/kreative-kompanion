Lesson 383 (STU-PE1 pe1-stage, 1 draft + 1 review repair): The canvas class used an undefined K, called byName('default'), read joint.pos, inverted apply()'s defaults, drew handles with the half-size view and blended 'both' backwards; the review repair fixed half; Claude fixed the joint pick, apply(), the blend, the handle view and screenOf.

Rule: A browser unit needs a browser check (its e2e sections), not node --check.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), `wrong-*` (the Coder's last output), `corrected-*` (what was kept). Code files carry a .txt suffix so the lint gate skips them.
