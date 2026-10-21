# Lesson 401

Lesson 401 (STU-PE2 pe2-mannequin, Coder+Claude): The Coder wrote indices[i] instead of indices[b * 36 + i], so every box overwrote the first 36 indices and Blender showed one box. It passed because Claude test only checked each index < count. Test the exact layout of index buffers, not just bounds. 1-line fix.

Rule: Test the exact layout of index buffers (per box), not just that indices are in range.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
