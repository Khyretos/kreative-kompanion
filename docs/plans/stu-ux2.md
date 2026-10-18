# STU-UX2: Kreative Studio facelift, part 2 (forms, queue, players, polish)

Task `kompanion-STU-UX2` (spec in the Kompanion task list; copy in kreative-kompanion-tasks/queue-specs-2026-10-18.md).
The code lives in the Kreative Studio repo (kireserver `~/Docker/Personal-projects/remote-image-generation`, local
only): branch `stu-ux2`, commit ca03535, merged into main as 38c8b06 on Kees's "Approve" (2026-10-18 01:24Z) and
deployed with `docker compose up -d --build`. This file holds the run log, lessons and metrics.

## What changed

- **Forms:** the image form's fields sit in three groups (What to make, Describe it, Size) with small headings;
  inline help under the four prompt fields (linked with aria-describedby); an empty field gets its message right
  under it (red, with a badge), the field is marked aria-invalid and focused, and the message goes away as you type
  (`forms.js`). Rating, refine rating and video size are segmented controls with arrow-key support
  (`segmented.js`); the size buttons look the same. Advanced stays collapsible.
- **Queue:** a waiting card shows its place ("Next", "2nd in line", "Starting") and a running card a live m:ss timer
  as a chip over the thumbnail (`queue.js`); Cancel stays. A failed card shows a one-line reason ("Ran out of GPU
  memory", the last line of a traceback, at most 80 characters; the full text in the title) with Retry (puts the
  job back in the right tab and focuses Generate, for every kind) and Delete.
- **Players:** audio cards get a waveform player drawn from the file (play/pause, click to seek, time) on its own
  row above the OGG and WAV downloads (`players.js`); video shows its first frame as poster (`#t=0.1`), has a Loop
  toggle and a download link. Card controls share one height.
- **Polish:** one toast style for undo and errors, stacked at the bottom (`toast.js`; no more `alert()`);
  orange focus rings on dark, orange-ink on light, orange in the purple header; Ctrl+K focuses a library search that
  filters by prompt or type together with the type chips, `?` opens a keyboard map dialog (`keys.js`).
- **Fixes found on the way:** main.js's `escapeHtml` did not escape quotes (an error text with `"` broke its title
  attribute); two type buttons lacked `aria-checked` (Lighthouse).
- New files: `public/js/{queue,toast,forms,segmented,players,keys}.js`, `public/css/{ux2-forms,ux2}.css`.

## Tests

- `test/e2e/ux2.e2e.js` (new, written first, 25 sections; `UX2=<sections>` runs some): groups, help, fielderr,
  formstyle, validate, validate2, segments, toastapi, waveapi, queue, elapsed, failed, redoapi, retry, audio, video,
  controls, toast, focus (both themes), taborder, keys, keymap, contrast (every visible text in five tabs, the
  drawer and the keyboard map, both themes, WCAG AA), phone (390 px, both themes, no sideways scroll), polish.
  On the old site 17 of the first 18 sections failed (controls already passed); all 25 pass now.
- `test/e2e/lighthouse.e2e.js` (new): Lighthouse accessibility, dark and light, 390 and 1300 px: 100 in all four
  (the old site scored 95: aria-required-attr on the type buttons).
- `test/e2e/ux2-shots.e2e.js` (new): the approval screenshots. `run.sh` passes `UX2` and mounts `$LH` and `$OUT`.
- UX1, simple, delete (now uses the new toast), shots and warnings browser tests and all 13 node test files pass.

## Screenshots

`stu-ux2/ux2-<width>-<theme>-{library,form,keymap-toasts}.png` at 390 and 1300 px, light and dark.

## Coder run log (one pipeline run per unit, up to 3 attempts; OVMS "Coder" and soucouyant's Ollama in parallel)

Every check failed on the tree before its unit. Checks and tools: `docs/qwen-situations/stu-ux2-unit-checks/`.

| Unit                      | File          | Model      | Attempts | Result                          | Claude's correction                   | Lesson   |
| ------------------------- | ------------- | ---------- | -------- | ------------------------------- | ------------------------------------- | -------- |
| 1 groups                  | index.html    | OVMS       | 1 + 3    | weak check passed, rerun failed | wrapped the three groups by script    | 273, 275 |
| 2 hints                   | index.html    | OVMS       | 1        | pass                            | none                                  |          |
| 3 search + keymap markup  | index.html    | OVMS       | 3 + 2    | pass on rerun (check was wrong) | none                                  | 274      |
| 4 forms.js                | new           | OVMS       | 3        | fail                            | 4 spots                               | 276, 277 |
| 5 segmented.js            | new           | soucouyant | 3        | fail                            | 3 spots                               | 278      |
| 6 queue.js                | new           | OVMS       | 3        | fail                            | 4 lines                               | 283      |
| 7 toast.js                | new           | soucouyant | 2        | pass                            | added the default timeout             | 279      |
| 8 players.js              | new           | OVMS       | 3        | fail                            | rewrote most of it                    | 284, 285 |
| 9 keys.js                 | new           | soucouyant | 3        | fail (check also wrong)         | 2 lines                               | 280, 281 |
| 10 submit validation      | main.js       | OVMS       | 1        | pass                            | none                                  |          |
| 11 form validation        | main.js       | OVMS       | 1        | pass                            | none                                  |          |
| 12 queue/fail card        | main.js       | OVMS       | 3        | fail                            | reverted 3 inventions, added 1 call   | 287      |
| 13 card players           | main.js       | OVMS       | 3        | fail (cause outside main.js)    | removed 2 pasted copies               | 288      |
| 14 redoJob                | library-ui.js | OVMS       | 1        | pass                            | none                                  |          |
| 15 retry + loop           | main.js       | OVMS       | 1        | pass                            | none                                  |          |
| 16 ux2-forms.css          | new           | soucouyant | 3        | fail                            | 3 spots                               | 282      |
| 17 ux2.css                | new           | soucouyant | 3        | fail (check also wrong)         | removed `@import` and `outline: none` | 286      |
| 18 delete toast           | main.js       | OVMS       | 2        | pass                            | none                                  |          |
| 19 no alert()             | main.js       | OVMS       | 1        | pass                            | none                                  |          |
| 20 escapeHtml quotes      | main.js       | OVMS       | 1        | pass                            | none                                  | 287      |
| 21 drawer ignores player  | library-ui.js | OVMS       | 1        | pass                            | none                                  | 288      |
| 22 failed reason one line | ux2.css       | OVMS       | 1        | pass                            | none                                  |          |
| 23 old toast CSS out      | style.css     | OVMS       | 1        | pass                            | none                                  |          |
| 24 aria-checked           | index.html    | OVMS       | 1        | pass                            | none                                  |          |
| 25 CSS polish             | ux2.css       | OVMS       | 3        | fail                            | 3 rules                               | 289, 290 |
| 26 audio row              | ux2.css       | OVMS       | 1        | pass                            | none                                  |          |

Units that passed as drafted: 13 of 26 (OVMS 12 of 19, soucouyant 1 of 6 plus unit 7 after one repair round);
units Claude corrected: 13. Coder totals (docs/qwen-log/stu-ux2.jsonl): 55 calls, 1,210 GPU seconds, 466,498 prompt
and 36,623 output tokens.

## Metrics (step 10)

| Item                                                                                                         | Value                                                                                  |
| ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| Claude, this thread (UX2, the UX1 leftovers, the engine answer and WIN-RESTART together; not split per task) | 81 answers, 61,809 output, 170 input, 36.4 M cache read, 105 k cache write (at 01:31Z) |
| Coder (OVMS + soucouyant)                                                                                    | 55 calls, 1,210 GPU s, 466.5 k prompt, 36.6 k output tokens                            |
| Other local models                                                                                           | none (no PR-Agent on the Studio repo)                                                  |
| Review / CI rounds                                                                                           | none: the Studio repo has no CI; every check ran on kireserver in kk-sandbox           |
| Findings                                                                                                     | unmeasured (no PR-Agent on the Studio repo)                                            |
| Time worked vs waiting                                                                                       | about 75 min from the spec to the screenshots; Kees approved 4 min later               |

## Merged, deployed, live check

Kees approved the look (2026-10-18 01:24Z, "Approve"). Merged into the Studio's main as 38c8b06 and deployed.
Live check against the running container (read-only, a fresh user on the docker network, nothing sent: no POST),
1300 px dark and 390 px light: all new scripts and styles load, no page errors and no failed requests, three
field groups, the rating segments, the inline error on an empty prompt, Ctrl+K focuses the search, `?` opens the
keyboard map, no sideways scroll.

## Not done yet

- Brand docs (Outline, Windshift BRAND) for UX2: the docs thread publishes the UX1 text; UX2 adds forms, queue chips,
  players, toasts and focus rings to it.
- The task row `kompanion-STU-UX2` is still queued: setting it done is a live-data write (Kees's word).
