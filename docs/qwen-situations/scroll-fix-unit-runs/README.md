# SCROLL-FIX unit runs (2026-10-12)

Kees (14:13Z): editing a task, the panel does not scroll, so the bottom buttons are reachable only on a tall window. Audit of every panel, form and dialog: `web/tests/scroll.spec.ts` opens 10 views (new task form, edit task, settings, capabilities, assets, studio, search dialog, chat menu, machines, access) at 1200x380 and 390x420, light and dark, and checks that every control can be reached by a user's scrolling (a box with `overflow: hidden` does not count, because only script can scroll it). Closed `<details>` content and the virtualized asset grid are skipped.

Found, then fixed:

- The New task / Edit task form (`.task-editor`) sat in the right panel with no scrolling at all: its fields and the Add task button were cut off on a short window.
- The left sidebar on a short window: the project/chat list was squeezed to nothing between the fixed header and footer, so chats, projects and Sign out were out of reach. Below 640 px of height the whole sidebar scrolls instead.

All other views were already fine.

| Unit | Job | Model | Attempts | Result |
| --- | --- | --- | --- | --- |
| scf1 | two CSS edits in web/src/styles.css | OVMS | 1 | Coder, no correction |

The log is `scf1-ovms.log.jsonl.gz`; totals are in `docs/qwen-log/scroll-fix.jsonl`. The test failed before the edit and passes now (40 cases). The full Playwright suite passes (256).
