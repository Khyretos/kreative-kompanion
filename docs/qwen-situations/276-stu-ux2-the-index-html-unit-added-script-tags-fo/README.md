# 276: STU-UX2 (Claude, harness)

the index.html unit added script tags for files that did not exist yet; Express's catch-all served index.html for them, so every later check failed with "Unexpected token '<'" (units 4 and 5 lost attempts to it). Create empty placeholder files for scripts/styles a markup unit references before running later units.

A check or harness mistake by Claude: no Coder output to rerun; the fixed check is in ../stu-ux2-unit-checks/.
