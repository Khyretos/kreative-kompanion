# CI-MERGE: merge a pull request by itself

Before: Kees merged every PR by hand after CI and the review gate passed.
After: the `auto-merge` workflow merges a PR through the Forgejo API once the six required checks are green and `tools/qwen/review_findings.py` exits 0.

## How it works

- `.forgejo/workflows/auto-merge.yml` runs on PR events, on every comment (PR-Agent's review, the lint summary) and every 10 minutes. It looks at each open PR once and exits; it never waits, because the kireserver runner has capacity 1 and a waiting job would block the build jobs it waits for.
- `tools/auto_merge.py` decides per PR: skip (closed, draft or `WIP:`/`Draft:` title, label `no-auto-merge`, fork, other base), wait (a required check missing or pending, review pending, head moved, 409), blocked (a check failed, gate exit 1 or unknown, conflicts, merge refused), else merge with `head_commit_id` set to the checked head, so a push during the gate cannot slip in.
- The gate runs main's `review_findings.py` against a worktree of the PR head (`REVIEW_REPO`), with `FORGEJO_TOKEN` as the read token and without the merge token in its environment.
- Settings: `.forgejo/auto-merge.json` (required checks, opt-out label). Run `python3 tools/auto_merge.py --dry-run` with `FORGEJO_API`, `REPO` and `READ_TOKEN` to see the decisions without merging.
- Needs the repo secret `AUTO_MERGE_TOKEN` (a Forgejo token of the repo owner with write:repository and write:issue). Without it the job only prints dry-run decisions. A merge made with the job token would start no CI on main, hence the user token.
- Not done: a deploy after the merge. `tools/deploy.sh` needs the Docker socket on kireserver and a live check as the test user, which a CI job cannot give; Kees or a task thread keeps deploying.

## Metrics

See the task reply and the summary line in `docs/qwen-log/ci-auto-merge.jsonl`.
