# JOBS-AFTER-UPDATE: jobs handed out with a runner update are no longer lost

Before: the server hands a runner up to 10 queued jobs per poll and marks them all sent. A runner that gets a
`self_update` installs it and exits at once, so any job after it in the same batch (for example a grant Kees added
right after pressing Update) stayed "sent" forever and never ran.

After: `take_jobs` ends the batch at a `self_update`. Later jobs stay queued and go out on the restarted runner's next
poll.

Found while checking why Kees's `C:\kk-runner` grant for DESKTOP-387PPHP (2026-10-18 ~03:35Z) never reached the PC.
Not proven to be that case: the task database read that would show the job's state is waiting on Kees.

## Checks

- `server/src/access/runner_tests.rs` (written first; 2 of its 3 tests failed on the old code): a grant after an
  update waits for the next poll, an update first goes out alone, without an update all jobs go out oldest first.

## Coder run log (OVMS on kireserver, one pipeline run per unit, up to 3 attempts)

| Unit      | File                        | Attempts | Result | Claude's correction |
| --------- | --------------------------- | -------- | ------ | ------------------- |
| take_jobs | server/src/access/runner.rs | 1        | pass   | none                |

Claude wired the test module (`#[path]`, 3 lines). Metrics: Coder 1 call, 15 GPU s, 6,817 prompt and 392 output
tokens (docs/qwen-log/jobs-after-update.jsonl); Claude unmeasured for this fix alone; no new lessons.
