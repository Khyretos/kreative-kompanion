# Lesson 397

Lesson 397 (STU-PE2 pe2-bvh, Coder): jointNames returned joint objects instead of names with the finger filter inverted; toBVH(clip) crashed without opts; need() at the top level (browser load order, lesson 386 again); MOTION wrote one line per joint and skipped joints missing from the pose; invented V.inv and V.frame instead of the FK.local the prompt named. Two patch rounds of 114/359 tokens. Claude fixed about 30 lines.

Rule: Name the exact helpers and return shapes (names, not joint objects) and repeat the lazy need() rule in every job; a repair round that never touches the reported error is a sign to stop patching.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
