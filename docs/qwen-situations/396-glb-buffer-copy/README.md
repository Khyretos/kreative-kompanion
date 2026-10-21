# Lesson 396

Lesson 396 (STU-PE2 pe2-glb, Coder): Browser+Node file used Buffer.copy on a Uint8Array (use out.set), guessed the pad byte from the data instead of taking it per chunk (JSON 0x20, BIN 0x00), left the 8-byte chunk headers out of the total length and returned the bare JSON when bin was empty. Both patch rounds were 66-558 tokens and never touched copy. Claude fixed 8 lines.

Rule: Browser+Node files use only Uint8Array/DataView APIs (out.set, never Buffer.copy); give the pad byte per chunk as a parameter.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
