---
name: worker/web
description: Writing the vanilla TypeScript web app and its Playwright tests.
roles: [worker, reviewer]
tags: [typescript, web, css, html, playwright]
paths: ["web/**"]
---
# Worker: web app (vanilla TypeScript)

Topic lessons moved into cards (live-updates, playwright, css-layout, controls), loaded when a job needs them. Add new lessons to the card they belong to.

47. (2026-10-05) "Plain JavaScript" means no type annotations; run `node --check` on every `.mjs`.
48. (2026-10-06) Helpers that use a function's parameters or its elements (api, callbacks, the
    sheet's nodes) are closures inside that function, never module-level functions: a draft moved
    them out and every name was undefined. Await an API call before reading its result.
58. (2026-10-06) A patch replaces its whole SEARCH text: every line in SEARCH that you keep must be
    in REPLACE too. A draft searched the import lines to add one name and dropped another import;
    three fix rounds didn't restore it. Search only the line you change.
73. (2026-10-07) To give a `.map()` result a declared type, annotate the callback's return,
    `.map((w): StudioType => ({ ... }))`. Never cast mid-chain (`.map(...) as T[]` followed by
    `.sort(...)` on the next line): it does not parse (TS1128), and four fix rounds did not see why.

- (2026-10-18, lessons 267-271) In a template literal, an inserted `${...}` goes exactly where the task says, never inside an attribute string. Edit only the lines the task names. Delegated click handlers use `e.target.closest(selector)`. Never declare the same name twice in one scope; in a repair round, change the broken lines and do not add a second copy of a block.
