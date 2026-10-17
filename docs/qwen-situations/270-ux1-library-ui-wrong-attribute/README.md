# 270: library-ui.js used the wrong card attribute (STU-UX1 unit 9)

New file (~85 lines): skeletons, the details drawer, the empty-state example. After 3 attempts: cards were
looked up by `data-id` (the cards carry `data-prompt-id`; the prompt did not name it: Claude's prompt
fault), the load/error handlers tested the class on the `<img>` instead of its `.job-thumb`, drawer clicks
read `e.target.dataset` (a click lands on the svg or span), the observer block was duplicated by a repair
round (a SyntaxError), and the whole drawer body was nested inside `.drawer-head`. Claude fixed those
34 lines; the Coder's structure stays. Files: `wrong-library-ui.js`, `corrected-library-ui.js`, `job.json`,
`trace.tgz`.
