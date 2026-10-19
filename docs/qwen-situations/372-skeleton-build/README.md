Lesson 372 (STU-PE1 pe1-skeleton, 2 prompts, 3 attempts): First prompt: the 62-joint table was unrolled literally and hit the 6000-token cap. Compact prompt: r_ twins for centre joints too, right wrist never mirrored, twin aim not mirrored, limit rows left as arrays, the PALM front never resolved, V.scale(u, t) with swapped arguments, hand names off by one; Claude rewrote the 45-line build section.

Rule: When a table is expanded (twins, mirrored rows), give the loop itself and the helper signatures (scale(vector, number)).

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), wrong-* (the Coder's last output), corrected-* (what was kept). Code files carry a .txt suffix so the lint gate skips them.
