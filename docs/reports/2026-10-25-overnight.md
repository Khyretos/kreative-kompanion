# Overnight report, 2026-10-25

Kees went to bed at 01:55Z with the order: get as far as possible, take the
recommended option for each decision, report in the morning. Newest first.

## PR 287: compact UI (from his phone review)

- Root cause of the green dot pulsing with nothing happening: the 6 "running"
  tasks were Windshift board/import items that Kompanion never ran. The chip now
  counts only tasks with a live run and pulses only on activity in the last 30 s.
- Task panel: one row of small filter chips; rows say now / ago / stalled /
  "on the board, not run here".
- Command cards: one header row, bash-highlighted command, Explain on every card.
- Copy icon next to the message time; computer picker on one row.
- CI: the server job failed once on a commit that only added the findings JSON
  (same code passed one commit earlier, 676 tests pass locally); this report is the
  next commit, which re-runs it.

## Decisions taken

- Explain needs no setting: the click is the opt-in, the answer is cached.
- Export button hidden on phones so the top bar fits 390 px (still on desktop).
