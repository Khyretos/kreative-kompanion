# 273: STU-UX2 (Claude, check)

the "groups" check only counted fieldsets with legends; unit 1's first draft changed only the opening tags, so the groups nested inside each other and a comment was dropped, and it still passed. A check for a wrapping edit must assert structure: parent, no nesting, which elements sit in which group, comments kept.

A check or harness mistake by Claude: no Coder output to rerun; the fixed check is in ../stu-ux2-unit-checks/.
