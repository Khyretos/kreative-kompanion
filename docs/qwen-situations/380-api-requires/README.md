Lesson 380 (STU-PE1 pe1-api, 3 attempts): It invented a need(name, file) loader with paths relative to the wrong folder and named exports that do not exist, and listed saved poses by looking for files named meta.json in the user folder; the repair rounds changed nothing; Claude wrote 8 requires and fixed the listing.

Rule: For a CommonJS file give the exact require lines; never let the model invent a loader.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), wrong-* (the Coder's last output), corrected-* (what was kept). Code files carry a .txt suffix so the lint gate skips them.
