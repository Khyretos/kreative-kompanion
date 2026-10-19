Lesson 408 (STU-PE3 pe3-fit): posefit.js draft (1 attempt + 2 patch rounds) did not load: it exported fit and regions that it had put on an object literal, scored every bone by the torso error, read limits from OPS.byName (does not exist) and used an undefined p for the ops. Claude rewrote it to the spec.

Files: job.json (the pipeline job; its check needs the test trees at /tmp/kk-pe3 and the kreative-studio:local or kk-anatomy:local image), `wrong-*` (the Coder's first full draft), `corrected-*` (what was kept). Code files carry a .txt suffix so the lint gate skips them.
