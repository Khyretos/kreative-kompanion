# Lesson 406

Lesson 406 (STU-PE2 pe2-ui, Claude): UI units were checked with node --check and grep only, so an undefined identifier and broken HTML nesting surfaced only in the e2e; Claude spec also gave the walk chip the pose-chip class that the PE1 e2e counts. Browser units get a no-undef lint and an HTML tag-balance check, and every existing e2e runs before a UI unit counts as passed; new UI reuses no class another test counts.

Rule: Repeat of the PE1 rule: browser units are checked in a browser, never with node --check or grep alone; add a no-undef lint and the full e2e set to UI unit checks, and new UI reuses no class another test counts.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
