# STU-UX1: Kreative Studio facelift, part 1 (shell, library, lightbox)

Task `kompanion-STU-UX1` (spec in the Kompanion task list). The code lives in the Kreative Studio repo
(kireserver `~/Docker/Personal-projects/remote-image-generation`, local only, never pushed), branch
`stu-ux1`, commit 2fe6b94. This file holds the run log, lessons and metrics.

## What changed

- Header: a Kompanion link next to the user chip; the header was already sticky.
- Library cards: one square thumbnail (object-fit cover), the type badge plus a status badge
  (Queued, Making, Failed, Cancelled), a two-line prompt and one row of icon actions, so every card has the
  same height. A skeleton shimmer shows until the picture has loaded. A progress ring replaces the
  progress bar on running jobs and moves without re-rendering the card.
- Details drawer: a click or Enter on a card opens it with the picture, the whole prompt, the type, seed and
  age, and Redo (puts the prompt and type back in the form and focuses Generate), Download, Kompanion and
  Delete. At 1200 px and wider it is a third column beside the library; below that it is a sheet on the
  right edge (full width on a phone). Esc or the close button closes it.
- Lightbox (new `lightbox.js`): previous/next buttons and Left/Right step through the visible pictures, a
  zoom button and +/- zoom to 2x, Esc or a click outside closes.
- Empty library: a box with "Try an example", which fills the prompt.
- Motion only on transform and opacity (170 ms), nothing with prefers-reduced-motion; WCAG AA (4.5:1) is
  checked on the badges, prompt, Kompanion link and drawer text in both themes; no sideways scroll at 390 px.
- New files: `public/js/library-ui.js`, `public/js/lightbox.js`, `public/css/ux1.css`,
  `public/css/ux1-cards.css`; `main.js` lost the old lightbox and got the badge, ring and empty-state
  helpers.

Not part of this change: brand docs in Outline and Windshift BRAND (spec step 6) wait for Kees's OK on the
look, because they describe it.

## Tests

- `test/e2e/ux1.e2e.js` (new, written first, 18 sections; `UX1=<sections>` runs some): all fail on the old
  site, all pass now.
- `test/e2e/ux-shots.e2e.js` (new): screenshots at 390, 768 and 1300 px, light and dark, with a seeded
  library (stub Kompanion, generated gradient pictures).
- The 4 old browser tests (simple, delete, shots, warnings) and all 13 node test files pass.
- `test/e2e/run.sh` now passes `UX1` and `SHOTS` into the container.

## Screenshots

Before (old site) and after, 1300 px dark with the drawer and 390 px light with the drawer:
`stu-ux1/before-studio-<width>-<theme>.png`, `stu-ux1/after-studio-<width>-<theme>.png`,
`stu-ux1/after-studio-1300-dark-drawer.png`, `stu-ux1/after-studio-390-light-drawer.png`.

## Coder run log (OVMS on kireserver, one pipeline run per unit, up to 3 attempts)

Every check was first run on the tree before its unit (must fail) and after Claude's reference version of
the unit (must pass): 12 of 12 behaved (lesson 221). The checks and tools are in
`docs/qwen-situations/stu-ux1-unit-checks/`.

| Unit | File | Attempts | Result | Drafted | Claude's correction | Lesson |
|---|---|---|---|---|---|---|
| 1 helpers | main.js | 1 | pass | 1 patch block | none | |
| 2 badge | main.js | 1 | pass | 1 patch block | none | |
| 3 thumb | main.js | 3 | fail | 1 patch block | moved the ring after `${media}`, restored a helper a repair round broke | 267 |
| 4 nobar | main.js | 1 | pass, but broke a neighbour | 3 patch blocks | restored the ring's condition | 268 |
| 5 tab | main.js | 1 | pass | 1 patch block | none | |
| 6 empty | main.js | 1 | pass | 1 patch block | none | |
| 7 ring update | main.js | 1 | pass | 1 patch block | none | |
| 8 old lightbox out | main.js | 2 | pass | 1 deletion | put the comment line back | 269 |
| 9 library-ui.js | new | 3 | fail | 210 | 54 drafted lines dropped, 34 written (190 kept: 82 %) | 270 |
| 10 lightbox.js | new | 3 | fail | 223 | 72 dropped (duplicates), 22 written (173 kept: 87 %) | 271 |
| 11 ux1.css | new | 1 | pass | 161 | 2 dropped, 6 written (one forbidden transition, one label rule) | 272 |
| 12 ux1-cards.css | new | 3 | fail (tree poisoned by unit 4) | 247 | 27 dropped (duplicates, scoping), 5 written | 272 |

Coder totals (docs/qwen-log/stu-ux1.jsonl): 20 logged calls, 1,306 GPU seconds, 191,672 prompt tokens,
17,788 output tokens. Units that passed as drafted: 6 of 12; units Claude corrected: 6 (two of them only a
line or two).

## Metrics (step 10)

| Item | Value |
|---|---|
| Claude, this task (from 22:41Z) | 112 answers, 113,379 output, 236 input, 38.1 M cache read, 197 k cache write |
| Coder (OVMS), drafts and repairs | 20 calls, 1,306 GPU s, 191.7 k prompt, 17.8 k output tokens |
| Other local models | none |
| Review / CI rounds | none: the Studio repo has no CI; checks ran on kireserver |
| Findings | unmeasured (no PR-Agent on the Studio repo) |
| Time worked vs waiting | unmeasured |

## Not done yet

- Kees approves the look (the spec's "done when"); then merge `stu-ux1` into the Studio's main, deploy with
  `docker compose up -d --build` in its folder, live check as the test user, and update the brand docs.
