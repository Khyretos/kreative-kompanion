# Overnight report, 2026-10-25

Kees went to bed at 01:55Z with the order: get as far as possible, take the
recommended option for each decision, report in the morning. Newest first.

## Morning status (12:06Z)

- Live: main a125369 (PRs 289, 290, 291, 294) deployed at 12:05Z; the container is
  healthy, /api/status answers, no warnings in the log, no new migrations.
- Still in CI with main merged in: 295 (review-gate concurrency), 296 (this report) and
  297 (Gemini and Mistral presets, see below). They merge on their own; 297 goes live
  with the next deploy.
- CI was the bottleneck: no Kompanion job started between 05:50Z and about 11:00Z. Every
  Linux runner slot was on kk-engine jobs that came first in pick-up order. With Kees's
  OK ("reorder CI") the Kompanion PR jobs went to the front on the CI page, right behind
  kk-engine PR 108's fix pipeline, as agreed with the engine thread.
- The review gate had not run since 03:30Z: 116 runs sat in one concurrency queue
  ("review-gate"), and only the oldest may run. PR 295 gives each PR its own group and
  lets a newer event replace a waiting one. Forgejo 15 has no way to cancel the old runs
  (no API, and the CI page has no cancel), so they drain one by one.

## PR 297: Gemini and Mistral presets

Kees doesn't need xAI now; he wants Kompanion able to use any frontier provider. Settings
already had presets for xAI Grok, OpenAI, Claude, DeepSeek, OpenRouter and Ollama, plus
"Other" for any OpenAI-compatible API. Added: Google Gemini (its OpenAI-compatible
endpoint, key in GEMINI_API_KEY) and Mistral (key in MISTRAL_API_KEY). The context meter
knows Mistral (128k) and Codestral/Devstral (256k) windows without a setting.

## PR 294: fixes from a real-model dogfood

A throwaway Kompanion (the deployed image) with Claude as orchestrator and the local Coder
(via ovms-queue) as worker and reviewer, a paired runner and a small fixture repo. Three
scenarios the way Kees works:

1. Fix a function in the middle of a 918-line file and add a test: done and correct, but
   16 steps and 22 calls. Causes: the run refused the test the task asked for, the worker
   then wrote it with `cat >> test_big.py` (and the review gate missed that); and the cut
   read only offered byte offsets, so the whole file was read again (same cut).
2. Use a fact from an earlier chat in a new one: the chat did not know the project's
   folder, searched the computer, was refused and ended on "Now I'll find the project folder."
3. Start two sub-tasks from a chat and report: clean (2 calls, 19 s).

Fixed: the test protection holds against the shell (and the gate counts shell writes);
test files the description asks to change may change; a cut read names the missing lines
and how to read them; project chats get the folder, computer and build/test commands; an
answer that only announces its next step gets one nudge.

## PR 291: reliability for long runs

- Runs left "running" by a server restart were never touched (they stayed running for
  ever). Now they are marked interrupted at startup and the task starts again after 30 s
  on the same computer and folder with the same settings; twice within the hour and it
  waits for you instead.
- Tool steps and task-run calls retry a dropped line or a busy provider (429/5xx, up to
  three tries, Retry-After respected); errors name the provider, the status and what it
  means ("too many requests, or the credit ran out"). Chat answers already retried.
- Every place that cut text for you or a model now says how much it left out.

## PR 290: artifacts pane

Long code (20+ lines) and HTML, SVG and Markdown blocks show a few lines in the chat with
Open/Preview; the pane (side panel on desktop, sheet on phones) shows them whole with
Preview/Code, Copy and Download. HTML runs in a sandboxed frame with its own policy: its
scripts work, but it has no network and no access to the app or your login. Files a step
wrote or read open from the step card.

## PR 295: review-gate concurrency

Every PR comment queued a review-gate run in one shared group (45 waiting while the runners
were short). Now one run per PR, and a newer event replaces the waiting one; each run sweeps
all open PRs anyway.

## Decisions taken (recommended option each time)

- Explain needs no setting: the click is the opt-in, the answer is cached.
- Export button hidden on phones so the top bar fits 390 px.
- The artifact pane overlays the chat on desktop (like a dialog) instead of squeezing the
  three columns; Escape or × closes it.
- A test file the task explicitly asks to change is allowed; any other stays protected.
- After a restart a run starts again from the folder as it is (a fresh plan), not from
  the middle of the old plan.
- CI was starved for hours by kk-engine release builds (another thread); I did not touch
  the runners or the CI page.

## PR 289: self-audit, compact approval cards and project overview

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
