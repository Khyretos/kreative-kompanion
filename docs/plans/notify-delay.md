# NOTIFY-DELAY: the test notification waits and counts down (2026-10-12)

Before: the "Send test notification" button sent at once; macOS shows no banner for the frontmost app, so on the Mac nothing appeared.
After: the button counts down 5 s ("Switch to another app now: the test notification comes in 5...") and then sends; real task notifications are unchanged. Tests first: 1 new Playwright test (countdown, nothing sent during it, one sent after) and delay 0 in the 4 existing button tests (`window.__kkTestDelaySeconds`); 9 desknotify tests pass, tsc clean.

## Coder run log (new loop: one draft, up to 3 repairs, models in parallel)

| unit                                            | model      | attempts | result                                                                       |
| ----------------------------------------------- | ---------- | -------- | ---------------------------------------------------------------------------- |
| nd-send-test (countdown in `sendTest`)          | soucouyant | 1        | pass, kept                                                                   |
| nd-main-tick (`main.ts` action passes the tick) | OVMS       | 4        | fail: returned the line unchanged; Claude pasted the given line (lesson 192) |

Situation: docs/qwen-situations/192-silent-check-no-repair-signal/. Claude tokens and the capability row are in the next report row (same thread as M6-11, GAMEPLAN-LIVE and the Mac work).
