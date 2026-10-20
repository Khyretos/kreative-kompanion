# Lesson 398

Lesson 398 (STU-PE2 pe2-bvh, Claude): Claude spec and test put BVH MOTION columns in skeleton order (`l_` and `r_` interleaved); BVH needs the hierarchy depth-first order. Collect the order while writing HIERARCHY. Check file-format rules against the format, not against our own list order.

Rule: Check file-format rules (column order, line layout) against the format itself, not against our own list order.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
