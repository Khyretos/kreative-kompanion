# STU-N2 work in progress: Coder runs, observed mistakes, situations (2026-10-12)

Written when the thread stopped at ~100 turns. Everything here is raw material for lessons and one-shot reruns (HANDOFF "00." rule).

## Per-unit Coder runs (3 independent runs from the same clean base, `units/jobs.json`, logs `units/run{1,2,3}.jsonl`, diffs `units/coder-diffs/`)

The base for each run was origin/main with Claude's production hunks removed (Claude's own versions are the files in the worktree).
`units/run3.sh` ran the three runs one after another on kireserver (/tmp/kk-n2c-r1..3).

| unit (file) | run 1 | run 2 | run 3 | what went wrong |
|---|---|---|---|---|
| cast-rust-meta (workflow.rs) | patch failed twice (SEARCH not in the file) | passed, 268 tokens, one doc comment on ONE long line (spec said two) | check passed WITHOUT a draft: stale cargo build (see below), not a real pass | focus excerpt cut before the end of the struct in 2 runs |
| cast-http (http.ts) | 4 attempts, tsc error: `cast` not in `StudioType` | same | same | my job order: the interface change lives in studio.ts (a later unit); shared type = dependent units |
| cast-studio-view (studio.ts) | passed first try, 1018 tokens | 4 attempts failed | 4 attempts failed | 2 of 3 runs broke tsc and then poisoned every later tsc check in the same tree |
| cast-mock (mock.ts) | passed first try | check failed (tree poisoned by studio.ts) | same | shared-tree poisoning |
| cast-main-state | passed after 1 repair round | tree poisoned | tree poisoned | same |
| cast-main-actions | passed after 1 repair round | tree poisoned | tree poisoned | same |
| cast-main-input | passed first try | tree poisoned | tree poisoned | same |
| cast-main-make | 4 attempts failed: `castProblem` called on the array (`studioCast.castProblem(...)`), a `string \| number` argument | tree poisoned | tree poisoned | real Coder mistakes in run 1 |
| cast-css | passed first try | passed | passed | none (CSS rules written from the description) |

Takeaways to turn into lessons (not yet filed): (1) a job on an EXISTING file needs `"mode": "patch"`: the first batch lacked it and replaced four whole files with short fragments (Claude's job-file mistake); (2) units that share a type or a tree must run in dependency order and in separate pipeline calls (rules 26/57); (3) a cargo check in a docker container with the SAME path (/src) and a shared target volume reuses the previous run's build when file mtimes are older than it (rsync -a keeps them): false green in run 3. Use a target volume per run or `touch` the edited file; (4) re-run lessons 151/155: a focus that does not show the lines the prompt names makes the Coder invent SEARCH text.

## trace-call (a pipeline.py feature: KK_TRACE_DIR keeps every prompt and answer; not applied, test kept as `trace-call/test_trace.py.txt`)

Six runs (2 prompt versions x 3), all failed; `jobs-v1/v2/v3.json`.
- v1 asked to add `import itertools` at the top of the file, outside the focus excerpt: the Coder invented the import block in its SEARCH (3/3), `patch failed twice`.
- v2 (no import, new focus): 3/3 `no edit blocks found`. Recorded in `v2-r1-trace/`: the first answer copied the `// ...` marker into a REPLACE and wrapped the return value (`return (trace_call(...) or ask(...))`, a recursion), the repair round answered with prose and a shell command (`cat tools/qwen/pipeline.py`) instead of edit blocks.
- v3 (anchor lines quoted, single edit block): 3/3 applied but the Coder only added the call lines and never wrote the `trace_call` function itself (and a stray `trace_call(system, user, "", 0)` at the top of `ask`). The check failed and NO repair round ran, see next section.

## check-loop (a real defect found by the runs, fixed through the Coder, applied in tools/qwen/pipeline.py)

`check_loop` filtered the check errors to lines naming the edited file and did `break` when none did. A unit test prints its own file name, so every failing unit test got ZERO repair rounds: this is why old logs show repair rounds that did nothing. Spec + tests: `tools/qwen/test_check_loop.py` (Claude wrote the test first, 1 test failed). The Coder (3 runs, `check-loop/jobs.json`) fixed it 3/3 with the same change in 100 tokens, but also duplicated the next line (`fix = {...}` twice). Claude removed the duplicate (1 line of 2 drafted lines kept). Coder output: `check-loop/coder-output-r1-pipeline.py.txt`.

## Other observations (STU-N2, this thread)

- studio-cast.ts (new file, `web/src/views/`): Coder draft 85 lines in 1 job + 3 repair rounds that did nothing (the defect above). Logic (add/remove/move/params/problem) kept as drafted (about 60 lines); wrong: imports `../core/html.mjs` and `../core/escape.mjs` (does not exist), functions not exported, double escaping `esc(text)` inside the html template. Claude fixed those and split pure logic (studio-cast.ts, tested in node) from the html panel (studio-cast-panel.ts), because node tests cannot import extensionless modules.
- GATE-FIX: black formatting differed between my docker mirror (cytopia/black) and MegaLinter's black 26: docstring layout failed the lint gate once. Format with the same black version as MegaLinter.
- Images (kept for the gpu-studio skill): 2 people without framing words are fine (9/10); 3 people needed a wider canvas (1536x768), the words `3people, group shot, medium shot, everyone in frame` and a crowd negative; before that 3/10. Strips overlap 7 %, one text encode per person limited by ConditioningSetAreaPercentage, joined with ConditioningCombine; each person's text starts with `adult, mature, solo`.
