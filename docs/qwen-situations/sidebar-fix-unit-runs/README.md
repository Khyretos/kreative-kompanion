# SIDEBAR-FIX unit runs (2026-10-12)

Kees (12:07Z): at the minimum width the values on the right are clipped. Cause: the right panel's Machines tab (`.group`, `.machine`, `.gpu` are grids without `minmax(0, 1fr)` columns, the machine head and GPU summary never wrap) was wider than its 260 px panel, so the panel cut off the watts, percentages and GB values. The left sidebar rows already shrink their titles first (checked at 260, 300, 420 px and on a 390 px phone, both themes).

| Unit | Job | Model | Attempts | Result |
| --- | --- | --- | --- | --- |
| sbf1 | six one-line edits in web/src/styles.css | souc | 1 | Coder, no correction |

The log is `sbf1-souc.log.jsonl.gz`; totals are in `docs/qwen-log/sidebar-fix.jsonl`. The test was written first and failed before the edit: `web/tests/sidebar.spec.ts` (12 cases: left sidebar at 260/300/420 px and 390 px, right panel tasks and machines at 260 px, light and dark). The full Playwright suite passes (216).
