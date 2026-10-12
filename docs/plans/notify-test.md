# NOTIFY-TEST: Send test notification button (2026-10-12)

Before: nothing in Settings could send a notification on demand, and a failing desktop plugin was silent. After: Settings > On this device has a "Send test notification" button that sends one notification directly (no focus or task-state checks) and shows the result; plugin failures are logged and shown. Known limit: on macOS tauri-plugin-notification 2.5.1 ignores the native send result (`let _ = notification.show()`), so page-side logging cannot see a macOS delivery failure; check usernoted.

Tests first (web/tests/desknotify.spec.ts, 4 new Playwright tests, all failing before). Final: 8 desknotify tests pass; tsc clean; CI 8/8.

## Per-unit run log

This task ran under the earlier loop (3 runs per model per unit); each run = one draft plus repair rounds fed with its own errors (attempts = check runs, max 4). The new loop (one draft, up to 3 attempts, models in parallel) applies from the next unit.

| unit               | model      | run | attempts | result |
| ------------------ | ---------- | --- | -------- | ------ |
| nt-send-test       | OVMS       | 1   | 4        | fail   |
| nt-send-test       | OVMS       | 2   | 4        | pass   |
| nt-send-test       | OVMS       | 3   | 4        | fail   |
| nt-send-test       | soucouyant | 1   | 2        | pass   |
| nt-send-test       | soucouyant | 2   | 2        | pass   |
| nt-send-test       | soucouyant | 3   | 2        | pass   |
| nt-log-failures    | OVMS       | 1   | 2        | pass   |
| nt-log-failures    | OVMS       | 2   | 2        | pass   |
| nt-log-failures    | OVMS       | 3   | 2        | pass   |
| nt-log-failures    | soucouyant | 1   | 2        | pass   |
| nt-log-failures    | soucouyant | 2   | 2        | pass   |
| nt-log-failures    | soucouyant | 3   | 2        | pass   |
| nt-settings-button | OVMS       | 1   | 1        | pass   |
| nt-settings-button | OVMS       | 2   | 1        | pass   |
| nt-settings-button | OVMS       | 3   | 1        | pass   |
| nt-settings-button | soucouyant | 1   | 1        | pass   |
| nt-settings-button | soucouyant | 2   | 1        | pass   |
| nt-settings-button | soucouyant | 3   | 1        | pass   |
| nt-main-action     | OVMS       | 1   | 1        | fail   |
| nt-main-action     | OVMS       | 2   | 1        | fail   |
| nt-main-action     | OVMS       | 3   | 1        | fail   |
| nt-main-action     | soucouyant | 1   | 1        | fail   |
| nt-main-action     | soucouyant | 2   | 1        | fail   |
| nt-main-action     | soucouyant | 3   | 1        | fail   |

Accepted: nt-send-test OVMS run 2 (4 attempts), nt-log-failures OVMS run 1 (2), nt-settings-button OVMS run 1, nt-main-action OVMS run 3 (3 of 4 tests; its 4th test was a bug in the accepted sendTest).

## What Claude corrected

- `sendTest` browser path (accepted nt-send-test draft): sent the notification although the permission stayed denied. Replaced the block (ask when not granted, block when the answer is not "granted"). 1 block, about 6 lines. Lesson 187.
- `main.ts`: the Coder's added action line lost its indentation; re-indented (cosmetic).
- Claude's own mistake: nt-send-test had no behavioural check (tsc plus grep only), so the bug surfaced one unit later. Lesson 187.

## Situations and lessons

- Lesson 187, situation docs/qwen-situations/187-unit-without-behaviour-check/ (jobs, every run's log, wrong and corrected block, rerun note); row in docs/lessons-learned.md.
- Coder totals (jobs, GPU s, output tokens, prompt tokens): OVMS 22, 208, 5402, 136105; soucouyant 15, 108, 4150, 86013.
- Review: PR 186, CI 8/8 twice, PR-Agent 2 findings (both false positives: stored situation excerpts), checked; merged by Kees 14:31Z, deployed 14:35Z (main 56b02ae).
- Claude tokens: not separable from the M6-11 thread (one session did both): see docs/plans/m6-11.md for the thread total over the same window; time worked about 25 min from the coordinator's request (14:10Z) to "ready to merge" (14:31Z), of which about 10 min were waiting for CI.
