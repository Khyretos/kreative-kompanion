# Model benchmark fixtures (KG-08)

Extends `docs/plans/nightly.md` (one fixture, "make the test pass") to one fixture per task type. Each is a tiny
repo with a failing check and a "done when"; a run is a normal task run with the model under test as the worker.

| Type | Fixture task | Passes when |
|---|---|---|
| bugfix | `kompanion-w2-fixture`: the failing test | the test passes |
| tests | a function without tests: add tests covering two edge cases | `cargo test`/`pytest` runs the new tests, all green |
| docs | a README missing its flags table: document every CLI flag | a script finds each flag name in the README |
| refactor | rename a function used in 3 files | grep finds no old name, tests pass |
| feature | add one CLI option with a test | the test passes |

After a run per fixture, send the results for the model:

```bash
curl -s -X POST "$KOMPANION/api/capabilities/bench" -H "Cookie: $SESSION" -H 'content-type: application/json' \
  -d '{"provider":"ovms","model":"Coder","results":[{"type":"bugfix","passed":true},{"type":"tests","passed":false}]}'
```

`GET /api/capabilities` returns the matrix. A model gets a verdict per type after 3 tries: passes at 80 % or more,
fails at 40 % or less. Task runs add to the same counts, so the matrix keeps learning from real work.
