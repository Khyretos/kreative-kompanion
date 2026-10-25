# Overnight report, 2026-10-25

Kees went to bed at 01:55Z with the order: get as far as possible, take the
recommended option for each decision, report in the morning. Newest first.

## Self-audit: compact approval cards and project overview

I clicked through every new feature at 390 px (dark) and 1280 px (light) on demo data
(chat, context meter, effort menu, facts, all four right-hand tabs, step and approval
cards, task panel, all of Settings). Fixed:

- Approval cards were five rows plus three full-width stacked buttons. Now: header,
  the step and its computer on one line, the three buttons side by side. The "waiting
  for you" chip is gone on waiting cards (the header badge already says it).
- Step summaries showed raw backticks ("Read `Cargo.toml`"); they render as code now.
- The approval card's words (Approve, Always allow, Deny, "on <computer>", Needs,
  every state) were English only; they are in EN/ES/NL now.
- Project overview: the type chip sits next to the name instead of on its own row; the
  four stat tiles are one compact row (also on a phone); "9 min ago" no longer cut off;
  the share form (name, role, Add) is one row next to the owner chip.
- Settings, card colours: one row per kind on a phone (the label field carries the
  colour on its edge) instead of a preview tag on a row of its own.
- Two tests that measured an element a live redraw can replace now poll.

Still English only (older screens, not touched tonight): most of Settings, the
project overview and the code block buttons. Noted for a later PR.

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
