# 294: overflow hidden on a grid item cut off the cards

MACHINES-CARDS unit B (Claude's spec): the job asked for `overflow: hidden` on a card inside the pane's scrolling grid (`.task-groups`). Overflow hidden sets the item's automatic minimum height to 0, so the grid squashed every card and cut off its forms; the Coder followed the spec and every check passed. Never put overflow hidden on an item of a fixed-height grid or flex scroller; round a header band with its own top radius instead, and add a test that each card's scrollHeight equals its clientHeight.

Rerun: `job.json` is the first spec (with the overflow rule); `wrong-*` is what it produced, `corrected-*` the draft from the fixed spec (job mcards-b-css-r2 in docs/qwen-log/machines-cards.jsonl). The check is web/tests/machinecards.spec.ts ("no card cuts off its own forms").
