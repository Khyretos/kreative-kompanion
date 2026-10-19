Lesson 374 (STU-PE1 pe1-raster, 3 attempts): encodePNG pushed a Uint32Array buffer with spread (not iterable), nested arrays, a one-byte width, little-endian lengths and CRCs over the wrong bytes; Claude rewrote encodePNG around u32()/chunk() helpers (22 lines).

Rule: For a binary format give the helper code (u32 big-endian, chunk(type, data)) and let the model only call it.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), wrong-* (the Coder's last output), corrected-* (what was kept). Code files carry a .txt suffix so the lint gate skips them.
