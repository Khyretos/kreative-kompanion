# PROJ-OVERVIEW: a project overview when a project is opened

Spec: kreative-kompanion-tasks/proj-overview-spec.md (queue thread 10, 2026-10-13).

## What changed

- Migration 0132: `projects.about` (markdown), `links` (JSON list), `repo_git` (cached git remote and branch),
  `created_at` (backfilled from the oldest task event or chat message, else `updated_at`; a trigger fills it on insert).
- `GET /api/projects/{id}/overview` (server/src/overview.rs), owner only: header (name, description, about, type,
  kind, links, dates, owner) and five parts: where it lives (computer, online, OS, folder, git remote/branch read in
  the background through a shell job under the computer's grants, cached for 60 min, "unknown" while offline),
  access (computer grants that cover the repo folder, active MCP grants, knowledge, assets, build/test/run/deploy
  commands with their last result; deploy marked "never run from here"), progress (done/total, goals, running,
  needs you, in review, next 3 queued with ready first, last merged PR), activity (last 10 across task events, runs
  and chats) and AI use (models of the last 30 days: runs, tokens, model minutes). A failing part is null with its
  reason in `errors`.
- `PATCH /api/projects/{id}/settings` also takes name (1-80 chars), description (one line, 200 chars), about
  (20,000 bytes) and links (10, http/https, label 1-60); all checked before anything is written. A Windshift
  project keeps its name and description read-only here (edited in Windshift); about and links work.
- The planner, the game planner and project chats read `about` after the description (1,500 chars); the Overseer
  reads 300 chars of it per project.
- Web: the Overview at the top of the Tasks pane (views/overview.ts): header with chips and Edit, four tiles
  (2x2 on a phone), About clamped to 4 lines with Show more and an empty state, then Where it lives, Access,
  Build and test, Recent activity (5, More for 10), AI use and the old settings forms in a collapsed Settings
  section. Sections are open on a desktop and closed on a phone. Edit in place (name, description, about with a
  preview, links one per line), optimistic with revert on error. A sidebar project click opens its Overview (on a
  phone the Tasks pane), even while a chat of another project is open (`paneProjectId`).

## Tests

- cargo test: 565 passed (new: 14 in overview_tests.rs, 2 in projects.rs).
- tsc --noEmit clean; Playwright: all 278 passed, incl. tests/projoverview.spec.ts (7: desktop light/dark, edit
  without reload, refused edit reverts, empty About, phone light/dark 2x2 and no sideways scroll, live update).
- Five existing specs open the Settings section first (it is collapsed now).

## Coder run log (OVMS on kireserver, one unit per job, at most 3 attempts)

55 units: 39 kept as the Coder drafted them, 16 corrected by Claude. Per unit: drafts / check attempts / result.

| Unit                               | What                                              | Result                                                                                         |
| ---------------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| m1                                 | main.rs mod + route                               | first try                                                                                      |
| p1, p2                             | Link struct + SettingsBody fields; clean_links    | first try                                                                                      |
| o1                                 | migration 0132                                    | right first try; my check missed the sqlx rebuild (3 attempts wasted)                          |
| h1                                 | HostStats::os                                     | first try                                                                                      |
| o3                                 | covers()                                          | 6 attempts failed ("/" trimmed to ""); Claude corrected (lesson 230)                           |
| o4, o5                             | with_about, activity_what                         | first try (Claude: chars not bytes in with_about's "…" test)                                   |
| o6                                 | progress()                                        | 3 failed; Claude rewrote (231)                                                                 |
| o7                                 | parse_git()                                       | 3 failed on the credential strip; Claude corrected that part (232)                             |
| o8                                 | refresh_git()                                     | passed; review found no deadline; Claude added it (236)                                        |
| o9                                 | load()                                            | first try                                                                                      |
| o10, o11, o12                      | where/access/progress parts                       | 3 failed each; Claude rewrote (231, 234)                                                       |
| o13, o14, o15                      | activity/ai parts, the route                      | passed (o13 1, o14 2, o15 2 attempts); o14's first run died with an OVMS GPU crash             |
| p3                                 | PATCH handler                                     | 3 failed (Windshift case); Claude restructured (232)                                           |
| c1-c4                              | about in chat, Overseer, planner, game planner    | first try                                                                                      |
| w1, w2, w3                         | TS types, client, http                            | right first try; my checks failed them (regexes), w3's repair rounds then broke the file (234) |
| w4                                 | mock getOverview                                  | 3 failed (prose pseudo-code from my prompt); Claude fixed (233)                                |
| w5-w8, w10-w12, w15, w16, w18, w19 | mock PATCH, state, view helpers and parts         | first try                                                                                      |
| w9, w13                            | rightsLabel, about()                              | right first try; my ordered needles failed them                                                |
| w14                                | where()                                           | 3 failed (stopped after the first part); Claude completed (232)                                |
| w17                                | activity()                                        | passed; More never showed; Claude fixed (236)                                                  |
| w20, w22                           | renderOverview, tasks.ts                          | 3 failed each; Claude corrected (232)                                                          |
| w21, w23-w25, w30-w32              | panel, loader, render wiring, events, paneProject | first try (w32 3 attempts)                                                                     |
| w26                                | click actions                                     | right; a repair round removed an import; my spec gap blocked its test                          |
| w27                                | form submit                                       | 3 failed (wrong place, invented API); Claude wrote it (233)                                    |
| w28                                | project click                                     | one fix (pane only on a phone) by Claude; its fix round died with an OVMS crash                |
| w29                                | CSS                                               | layout right; it also redefined :root colours, removed by Claude (235)                         |

Lessons 230-237 (237 = my own check faults); situations in `docs/qwen-situations/230-*` to `236-*`; the per-unit
logs in docs/qwen-situations/proj-overview-unit-runs/.

## Metrics

|                                 |                                                                                                                              |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Claude (this session to the PR) | 297 answers, 222k output, 0.6k input, 106 M cache read, 0.48 M cache write                                                   |
| Coder (OVMS)                    | 100 jobs (drafts + repair rounds), 55 units, 39 kept as drafted, 16 corrected; 2,042 GPU s; 1.02 M prompt, 47k output tokens |
| Other local models              | none yet (PR-Agent review follows in CI)                                                                                     |
| OVMS                            | crashed twice on a GPU error (clWaitForEvents -14, restart count 11 -> 12) and restarted itself                              |
| Review and CI rounds            | see the PR                                                                                                                   |
| Time                            | about 75 min worked, most of it waiting on Coder units and cargo/Playwright runs                                             |
