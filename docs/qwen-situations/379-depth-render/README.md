Lesson 379 (STU-PE1 pe1-depth, 3 attempts (one cut off at 5000 tokens)): render() read view.R as rows of rows, called vec functions that do not exist, never scaled radii to pixels, re-projected every shape per pixel and mapped grey before the depth range was known; Claude rewrote render() (70 lines).

Rule: A per-pixel renderer is 3 units (shapes to screen, hit tests, buffer to grey), each with a numeric test.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), `wrong-*` (the Coder's last output), `corrected-*` (what was kept). Code files carry a .txt suffix so the lint gate skips them.
