Lesson 386 (STU-PE1 pe1-presets, 1 attempt (Node check passed)): In the browser branch it set KKPose.PRESETS/NAMES instead of KKPose.presets; Claude fixed one line and added test/pose-browser.test.js.

Rule: A dual Node/browser file is checked in both branches.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), wrong-* (the Coder's last output), corrected-* (what was kept). Code files carry a .txt suffix so the lint gate skips them.
