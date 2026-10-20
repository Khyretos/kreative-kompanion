# Lesson 404

Lesson 404 (STU-PE2 pe2-libui, Coder): anim-library.js called refresh() that did not exist, read card.dataset.id/name that createCard never set and wrapped toast options twice; the Coder fixed all three in one review-guided patch round (1 attempt).

Rule: Review notes that name each broken reference let the Coder fix a UI file in one patch round; keep using them before Claude edits.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
