# Lesson 192: a check that fails silently gives the repair rounds nothing to work with

Situation: NOTIFY-DELAY (2026-10-12), job `nd-main-tick` (jobs.json): in web/src/main.ts replace the whole `"desk-notify-test"` line by the given line. Check: `grep -q 'Switch to another app now' web/src/main.ts && tsc && Playwright`.
Wrong output (OVMS, 4 attempts, 4 calls): the Coder returned the line unchanged every time. The grep part of the check fails without printing anything, so the repair rounds got an empty error and had nothing to react to (runs/nd-main-tick-ovms-r1.*).
Corrected: Claude pasted the given replacement line (the prompt contained it verbatim), then tsc and all 9 desknotify tests passed. The first unit (`nd-send-test`, soucouyant) passed first try.
Lesson: a check must print why it failed (`grep -q X f || { echo "main.ts does not contain X"; exit 1; }`), otherwise repair rounds repeat the same answer. Rerun after model refinement: run jobs.json (nd-main-tick) with the printing check and see whether it applies the line.
