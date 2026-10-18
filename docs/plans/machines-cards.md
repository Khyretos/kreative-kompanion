# MACHINES-CARDS: one card per computer in the Access tab

Before: the Access tab listed every computer's grants and forms one after another under a small grey name, so Kees
could not tell which checkboxes and buttons belonged to which computer. Text typed into one computer's folder field
(and the expiry choice) was copied into every other computer's form on the next live refresh, including the hidden
`system` target of the system-rights forms.

After: each computer is a card with a 2px violet border and its name on a violet band (white text, 7:1 or more in
both themes), plus an OS chip. Inside are a "Folder access" block and a "System access" block; both forms carry the
computer's name in their labels (`Folder access on DESKTOP-387PPHP`) and say what they grant ("Grant folder
access"). Windows computers get the example `C:\Users\you\projects`. Every form has its own `data-id`
(`<machine>|folder`, `<machine>|system`), so the live re-mount keeps typed text on the right computer.

## Checks

- `web/tests/machinecards.spec.ts` (written first, failed on the old tree): card header per computer, labelled forms,
  Windows example, typed text stays put over a live refresh, header contrast and painted band (light and dark),
  no sideways scroll at 390 px, no card cuts off its forms.
- Full web suite: 291 passed; `rightscroll.spec.ts` failed once under full load and passed 4 of 4 alone.
- Screenshots (390 px dark/light, 1280 px light) sent to Kees for the look approval.

## Coder run log (OVMS on kireserver, one pipeline run per unit, up to 3 attempts)

| Unit                    | File                    | Attempts | Result | Claude's correction                                       | Lesson |
| ----------------------- | ----------------------- | -------- | ------ | --------------------------------------------------------- | ------ |
| A machineCard           | web/src/views/access.ts | 1        | pass   | none (spliced in by script)                               |        |
| B card CSS              | web/src/styles.css      | 1        | pass   | none, but Claude's spec asked for overflow hidden: redone | 294    |
| B card CSS (fixed spec) | web/src/styles.css      | 1        | pass   | one rule per line (mechanical, like the rest of the file) |        |

Claude applied the literal parts: the demo Windows machine (mock.ts), `os` passed to `renderAccess` (main.ts), its
type and the `isWindows` helper. Lessons 294-296 (294 has a situation in docs/qwen-situations/).

## Metrics

| Item               | Value                                                                                          |
| ------------------ | ---------------------------------------------------------------------------------------------- |
| Coder (OVMS)       | 3 calls, 133 GPU s, 37,051 prompt and 3,840 output tokens (docs/qwen-log/machines-cards.jsonl) |
| Lines              | 49 template lines and 9 CSS rules drafted, all kept                                            |
| Claude             | unmeasured for this task alone (queue thread 17)                                               |
| Other local models | PR-Agent review: see the PR                                                                    |
