# Lesson 395

Lesson 395 (STU-PE2 pe2-anim, Claude): Claude test expected 15/45 for the loop wrap segment; the spec formula gives 10/40. The Coder was right and burned 3 attempts plus 2 patch rounds on a wrong test. Derive every expected number from the spec formula (write the arithmetic in the test comment), not by eye.

Rule: Derive every expected number in a test from the spec formula and write the arithmetic in a comment; a wrong expectation burns all three Coder attempts.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
