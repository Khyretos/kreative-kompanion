Lesson 384 (STU-PE1 pe1-panel, 1 draft + 1 review repair (patches half failed)): setView dropped the other angle, the legend text doubled as state, slider values were never set, inline styles and a #999 placeholder failed contrast, every op was logged twice; Claude fixed them.

Rule: Keep UI state in variables the prompt names; one place logs.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe1/studio and the kreative-studio:local image), wrong-* (the Coder's last output), corrected-* (what was kept). Code files carry a .txt suffix so the lint gate skips them.
