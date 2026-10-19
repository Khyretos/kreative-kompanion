# Lesson 402

Lesson 402 (STU-PE2 pe2-cli, Coder): if (clip) instead of if (!clip) replaced a --clip file with an empty clip and left --poses with null; .json went to an unknown format json instead of frames; any --flag was accepted (no unknown-flag error) and the next argument was swallowed as its value; the frames object was written without JSON.stringify; checkClip result unused. Two patch rounds (310/665 tokens) did not find the inverted branch. Claude fixed about 25 lines.

Rule: CLI jobs list every flag and its kind (value or switch) and require an unknown-flag error; the test runs each branch (--clip and --poses) separately.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
