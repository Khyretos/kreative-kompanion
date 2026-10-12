# GATE-TOOLS: the integration gate's `tools` check (2026-10-12)

Before: the first real gate run (after PR 183, workflow_dispatch) failed on `tools` and printed only "Integration gate failed: tools". Reproduced by hand on kireserver: the job image has python3 but no Pillow, numpy or ffmpeg, so `test_draw_pose`, `test_video_compare` and `test_video_metrics` error out (116 tests, 3 errors). web and server passed.

After: `tools/gate-tools.sh` runs every test under tools/ in a node:22 container on the runner's Docker (apt installs git, ffmpeg, python3-pil, python3-numpy; the commit is fetched inside the container, like gate-server.sh); the gate config calls it; `integration_gate.main` prints the failing check output into the job log. 120 tests pass in that container (checked by hand on kireserver). No test was dropped.

## Coder run log (kireserver, OVMS Coder, tests written first by Claude and failing before)

| unit | runs | repair rounds | mistakes | lines drafted / kept |
|---|---|---|---|---|
| tools/gate-tools.sh (new file) | 3, identical output, first-try pass | 0 | 1 in 3/3 runs: a comment claims it mirrors a `tools` job in build.yml that does not exist (lesson 180) | 16 / 15 |
| .forgejo/integration-gate.json (`cmd`) | 3, identical | 0 | 0 | 1 / 1 |
| tools/integration_gate.py (`main` print) | 3, identical | 0 | 0 | 1 / 1 |

Claude's own mistakes (lessons 181 and the log/test rows in docs/lessons-learned.md): first plan was to narrow the tools list to what the runner can run (coordinator stopped it), and a new test wrote gate-task.json into the working directory.

Coder totals (docs/qwen-log/gate-tools-fix.jsonl): 9 jobs, 65 GPU s, 1,866 output tokens, 38,598 prompt tokens. Situations: docs/qwen-situations/180-comment-copies-example-claim/.
