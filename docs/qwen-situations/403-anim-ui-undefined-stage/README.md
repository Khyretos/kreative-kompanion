# Lesson 403

Lesson 403 (STU-PE2 pe2-animui, Coder): pose-anim.js used a stage variable it never defined (TypeError on every action). The review-guided repair (3 attempts, e2e check) moved everything into setup() and left els undefined, which broke more. Claude kept the first draft and added a 3-line lazy lookup of KKPoseEditor.stage.

Rule: A browser file that needs another file's DOMContentLoaded result looks it up when used (a lazy getter); say so in the prompt. When repairs make an e2e worse, keep the first draft and fix it.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
