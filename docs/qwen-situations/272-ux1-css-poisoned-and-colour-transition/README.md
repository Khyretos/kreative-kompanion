# 272: card CSS failed on a poisoned tree; colour transitions (STU-UX1 units 11-12)

Unit 12 (`ux1-cards.css`) failed its 3 attempts only because of unit 4's ring on every card (lesson 268);
with main.js fixed, the Coder's first CSS passed. Its repair rounds left duplicated `.lightbox-zoom` rules
and unscoped `.status-error`/`.status-running` selectors; unit 11 (`ux1.css`) passed but added a
`background-color` transition the spec forbids (the motion check did not cover `.kompanion-link`). Claude
removed the duplicates, scoped the selectors and dropped the transition. Files: `wrong-ux1-cards.css`,
`corrected-ux1-cards.css`, `coder-ux1.css`, `corrected-ux1.css`, `job.json`, `trace.tgz`.
