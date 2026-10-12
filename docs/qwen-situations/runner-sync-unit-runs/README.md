# RUNNER-SYNC unit runs (2026-10-12)

One Coder draft per unit, retried with its own check errors, at most 3 attempts in total (Kees, 14:24Z); OVMS (kireserver) and the soucouyant Ollama in parallel on different units. `<unit>-<model>.log.jsonl` is the pipeline log (draft line and every check attempt with its errors). Files with `-void-check-bug` are runs voided because Claude's own check script was wrong (lessons 200 and 202); they are kept for the record, not counted. Jobs: `docs/qwen-situations/<lesson>/…json`. Totals: `docs/qwen-log/runner-sync.jsonl` (last line, summary.py).

| Unit   | Job                                         | Model | Attempts                | Result                                        |
| ------ | ------------------------------------------- | ----- | ----------------------- | --------------------------------------------- |
| rsa1   | runner/Cargo.toml: sha2 dependency          | OVMS  | 1                       | Coder                                         |
| rsa2   | runner/src/main.rs: `mod selfupdate;`       | souc  | 1                       | Coder                                         |
| rsb1   | `sha_ok`                                    | OVMS  | 3 (failed)              | Claude fixed one line, lesson 201             |
| rsb2   | `install_binary`                            | souc  | 3 (failed)              | Claude rewrote the body, lesson 204           |
| rsb3   | `install_binary` retry on "Text file busy"  | OVMS  | 2                       | Coder                                         |
| rsc1   | server: `self_update` in the job allow-list | OVMS  | 1                       | Coder                                         |
| rsd1   | `update_from_server`                        | OVMS  | 3 (failed)              | Claude wrote the body, lesson 205             |
| rsd2   | tools/sync_runner.sh                        | OVMS  | 1                       | Coder                                         |
| rsd3   | tools/deploy.sh calls it                    | OVMS  | 1                       | Coder                                         |
| rsv1   | runner version 0.6.0                        | souc  | 1                       | Coder                                         |
| rsm1   | runner `self_update` job arm                | souc  | 1                       | Coder                                         |
| rsw1   | web mock: outdated runner on soucouyant     | souc  | 1                       | Coder                                         |
| rsw3   | web http.ts `updateRunner`                  | souc  | 3 (failed)              | Claude added the line, lesson 203             |
| rsw2   | web client.ts interface line                | souc  | 3 (failed)              | Claude replaced it, lesson 207                |
| rsw4   | web mock method                             | souc  | 2 (passed, wrong place) | Claude moved it, lesson 206                   |
| rsw5   | machines.ts button                          | souc  | 3 (passed, wrong shape) | Claude applied the intended edit, lesson 206  |
| rsw7   | machines.ts updating state + button         | OVMS  | 3 (failed)              | Claude moved the lines, lesson 208            |
| rsw6   | main.ts action                              | OVMS  | 1                       | Coder                                         |
| rsdoc1 | docs/runner-install.md section              | souc  | 1                       | Coder, Claude reworded one phrase, lesson 209 |

Tests (written by Claude first): `runner/src/selfupdate_tests.rs` (8 cases incl. a local HTTP server and a concurrent-fork stress test), `tools/test_sync_runner.sh`, `web/tests/runnersync.spec.ts`, `tools/e2e_runner_update.py` (fake server, real runner binary). Lessons 200 to 209.
