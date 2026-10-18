# 274: STU-UX2 (Claude, check)

html.js wanted `class="keymap-close"` exactly while the job asked for `class="secondary-btn keymap-close"`, so unit 3's three attempts were judged by a wrong check. Validate a check against the markup the job asks for (a positive fixture), not only fail-before.

A check or harness mistake by Claude: no Coder output to rerun; the fixed check is in ../stu-ux2-unit-checks/.
