# 271: lightbox.js repair rounds duplicated code (STU-UX1 unit 10)

New file (~80 lines): the lightbox with arrows, zoom and keys. After 3 attempts: `closeBtn` declared twice,
`show()` defined twice, the nav buttons got `lightbox-nav prev` instead of `lightbox-prev`, `iconBtn()`'s
string used as an element (`.outerHTML`), the data attributes joined with `&` and unquoted, and the prompt
read from the clamped text instead of the `title`. Claude changed 22 lines. Files: `wrong-lightbox.js`,
`corrected-lightbox.js`, `job.json`, `trace.tgz`.
