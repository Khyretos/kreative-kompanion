# PROJ-OVERVIEW-FIX (PR 205, main d0165c7, 2026-10-13)

Three follow-ups from the PROJ-OVERVIEW live check.

- Overview tiles: 2x2 by the width of the overview (CSS container query), not the window. The desktop Tasks pane is 327 px wide, so it now shows 2x2.
- Progress bar fill uses the primary plum (--brand), not the orange accent.
- PATCH /projects/{id}/settings validates everything (type, folder, computer) before the first UPDATE, so a refused change saves nothing.

## Live check (127.0.0.1:8095 as the test user, project "OVR live check")

- Refused change (bad computer, type chat, folder /tmp/x): HTTP 400, the project keeps type programming and its folder.
- Browser (Playwright, session cookie), desktop 1280 dark: overview 327 px wide, 2 tile columns, bar rgb(92, 57, 142). Phone 390 light: 2 columns, bar the same, no sideways scroll.

## Metrics

| What | Value |
|---|---|
| Coder (OVMS, kireserver) | 8 model calls: 2 CSS drafts kept as drafted (first try, 16.9 GPU s), settings fix 3 attempts failed and Claude corrected it (lesson 238). Totals 152.8 GPU s, 3,572 output / 79,545 prompt tokens |
| Lines | Coder CSS: 4 lines changed, all kept. Settings: about 40 lines drafted, 0 kept (Claude wrote about 30) |
| Tests written by Claude | 1 Rust (a_refused_settings_change_saves_nothing), 1 Playwright (tiles by width, plum bar) |
| Review | PR-Agent 8 findings over 4 rounds (2 Coder verdicts of bug were wrong), all false positives, checked by Claude; lint failed once (final newline in situation JSON) |
| Claude tokens | unmeasured (thread 11 totals not split per task) |
| Time | most of the wait was CI plus three review rounds; about 3 h wait for Kees on the safety check (db, deploy) |
| Note | the settings job log shows check_passed true after 3 attempts in a tree that was reused for the second try; the final code is Claude-written, so I count the unit as corrected |
