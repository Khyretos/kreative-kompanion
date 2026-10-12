# PROJ-ARCHIVE: archive and unarchive projects

Spec: kreative-kompanion-tasks/proj-archive-spec.md (queue thread 9, 2026-10-12/13).

## What changed

- Migration `0131_project_archived.sql`: `projects.archived_at TEXT` (NULL = active).
- `PATCH /api/projects/{id}/settings` takes `archived: true|false` (true keeps the first date); `GET /api/projects` returns `archivedAt`.
- The scheduler skips archived projects (the loop and `tick_project`); a task that is already running is not stopped.
- The Overseer's scan leaves archived projects out, and a task proposed by project name never lands in an archived project (by id it still does). Import does not touch `archived_at`.
- Sidebar: a ⋯ menu on every project (Archive project / Unarchive), always visible, 44 px at phone width. Archiving is optimistic and shows "Archived NAME · Undo" for about 6 s. "Show archived (N)" appears when N > 0 (remembered on the device); archived projects then come last, dimmed, with an Archived chip.
- The "All" task list and the chat "Move to project" list leave archived projects out unless they are shown; an open archived project shows an "Archived · Unarchive" bar in its panel.

## Tests

- cargo test: 543 passed (new: archive round trip with first date, owner only, Changed event, running task untouched; scheduler skips; Overseer scan and name lookup; import keeps the date).
- tsc clean; Playwright `tests/projarchive.spec.ts` 8/8 (archive + Undo without reload, toggle, Unarchive, All tasks and move picker, archived bar, 390 px light/dark with 44 px menu button, AA contrast of the dimmed name, no sideways scroll); full suite 269 passed.

## Coder run log (kireserver OVMS, one unit per job, up to 3 attempts)

| Unit | File | Drafts | Result | GPU s | Out tokens |
| --- | --- | --- | --- | --- | --- |
| s1 | migration 0131 | 1 | passed | 4.7 | 65 |
| s2a | api.rs Project field | 1 | passed | 7.0 | 145 |
| s2b | api.rs SELECT | 1 | passed | 4.7 | 123 |
| s3a | projects.rs SettingsBody | 1 | passed | 4.0 | 99 |
| s3b | projects.rs settings (run 1, my check hid the code lines) | 3 | failed (check fault, lesson 219) | 47.3 | 1134 |
| s3b | projects.rs settings (run 2) | 3 | failed, Claude corrected (lesson 220) | 22.8 | 482 |
| s4a | scheduler.rs tick_project | 1 | passed | 7.6 | 153 |
| s4b | scheduler.rs loop | 1 | passed | 5.4 | 109 |
| s5a | overseer.rs scan | 3 | first draft right; my check ran the next unit's test (lesson 221) | 112.8 | 3167 |
| s5b | overseer.rs name lookup | 1 | passed | 13.4 | 101 |
| w1 | types.ts | 1 | passed | 5.2 | 123 |
| w2a | client.ts | 1 | passed | 4.7 | 132 |
| w2b | http.ts | 1 | passed | 4.7 | 132 |
| w2c | mock.ts | 3 | failed, Claude removed extras (lesson 222) | 65.1 | 1227 |
| w3a | state.ts interface | 1 | passed | 5.0 | 121 |
| w3b | state.ts default | 1 | passed | 4.6 | 132 |
| w4a | sidebar.ts lists | 1 | passed (dropped one comment; Claude restored it) | 8.2 | 235 |
| w4b | sidebar.ts row + menu | 1 | passed | 21.4 | 644 |
| w4c | sidebar.ts toggle | 1 | passed | 4.0 | 107 |
| w4d | sidebar.ts move picker | 3 | failed, Claude removed extras (lesson 222) | 41.1 | 1051 |
| w6 | tasks.ts All scope | 3 | failed, Claude applied it (lesson 223) | 32.9 | 821 |
| w7 | projectpanel.ts bar | 1 | passed | 6.6 | 191 |
| w8 | styles.css | 1 | passed (put the block mid-file, not at the end; kept) | 19.8 | 578 |
| w5a | main.ts render keys | 1 | passed | 8.9 | 263 |
| w5c | main.ts outside click | 1 | passed | 9.7 | 296 |
| w5d | main.ts right keys | 1 | passed | 7.1 | 209 |
| w5e | main.ts archiveProject | 1 | passed; Claude fixed timer and rollback (lesson 224) | 23.1 | 715 |
| w5b | main.ts actions (full spec as check) | 1 | passed | 12.2 | 376 |

Totals: 27 units, 40 Coder jobs (drafts and repair rounds), 514 GPU seconds, 12,931 output tokens, 336,856 prompt tokens. 20 units passed as drafted, 5 were corrected by Claude (s3b, w2c, w4d, w6 and small leftovers in w5e), 2 failures were Claude's check faults (s3b run 1, s5a). Tests, checks, the spec and the lesson texts were written by Claude.

Logs: docs/qwen-situations/proj-archive-unit-runs/ (job files, gzipped logs, checks.tar.gz).
