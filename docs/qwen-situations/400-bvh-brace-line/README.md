# Lesson 400

Lesson 400 (STU-PE2 pe2-bvh, Claude): The BVH passed Claude own whitespace-token parser but Blender (line-based reader) refused `ROOT pelvis {` on one line. Claude spec showed the brace on its own line for End Site only. Test generated files with the real consumer (Blender in a capped container) before calling a unit done, and make the in-repo test check the line layout.

Rule: Before a unit counts as done, load the generated file with the real consumer (Blender in a capped container), and make the in-repo test check the line layout the consumer needs.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
