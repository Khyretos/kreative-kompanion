# GATE-FIX: the integration gate failed on "web" (2026-10-12)

## Cause

The first run of the gate (KG-09) on the KG-11 merge (main 08da2ad, Forgejo task 4811) failed in 2 seconds on
`web`. Three things were wrong, all in the gate's own config:

1. The job has no `container:`, so it runs in the runner's default image (`ghcr.io/catthehacker/ubuntu:act-24.04`).
   That image has no `web/node_modules`; `npx tsc --noEmit` therefore fetched the unrelated `tsc` package
   ("This is not the tsc command you are looking for") and exited 1. The runner was not missing anything
   that `npm ci` does not give it.
2. The `server` (`cargo test ... | tail -5`) and `tools` (`unittest ... | tail -5`) checks ended in a pipe
   to `tail`, so the exit code was tail's: both **passed vacuously** in the same 2 seconds. The image has no
   cargo at all. The gate could never have caught a server regression.
3. The tag `gate-ok` does not exist yet ("couldn't find remote ref"), which is fine: the first run counts all merges.

## Fix (CI config and one script; no GPU or container changes on the hosts)

- `web`: `cd web && npm ci && npm run typecheck` (same commands as the `web` job in build.yml).
- `tools`: the unittest command without the pipe.
- `server`: `sh tools/gate-server.sh` runs `cargo test --locked` for machine-stats and server in a
  `rust:1-alpine` container on the runner's Docker (DOCKER_HOST, as in build.yml's server job; the commit is
  fetched inside the container with the job token because a dind host cannot mount the job workspace).
- The workflow gets `workflow_dispatch` (runs with `--force`, so the gate can be tried without five merges) and
  the TOKEN/REPO_URL/SHA env.
- `tools/test_integration_gate.py`: two tests keep a pipe out of the check commands and keep `npm ci` in `web`.

## Checked

- `npm ci && npm run typecheck` in a clean `node:22` container: clean, exit 0.
- 9 gate unit tests pass; black, prettier and shfmt clean.
- Not checked locally: `tools/gate-server.sh` needs the runner's Docker and the job token. After the merge, run the
  gate once with "Run workflow" (workflow_dispatch) and read the result.

## Metrics (step 10)

Config-only change, so no Coder draft went through pipeline.py (step 4 skipped, not zero).
Claude for this PR (from 09:17Z, to the first push): 26 answers, 11,515 output tokens, 3.0 M cache read.
Local models, review and CI rounds: filled in when the PR is green.
