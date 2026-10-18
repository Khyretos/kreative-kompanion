# WIN-RESTART: the Windows runner restarts its own task after a self-update

Before: on DESKTOP-387PPHP the self-update to 0.6.2 swapped the binary and the runner exited with code 4, as on
Linux. Task Scheduler's RestartOnFailure (999 times, every minute) only fires when the task fails to launch, not on
a non-zero exit code, so the runner stayed offline until someone started the task by hand.

After: on Windows, right before that exit, the runner starts a detached `cmd /C "ping -n 4 127.0.0.1 >NUL &
schtasks /Run /TN KompanionRunner"`. The wait matters: the task's MultipleInstancesPolicy is IgnoreNew, so a `/Run`
while the old instance still runs would be ignored. Linux is unchanged (systemd restarts on exit 4).

Runner version 0.6.3. The fix acts in the runner that performs the update, so 0.6.2 machines still need one manual
start after updating to 0.6.3; from 0.6.3 on, updates restart by themselves.

## Code

- `runner/src/winsetup.rs`: `restart_line()` (tested on Linux: exact command, wait before the run).
- `runner/src/setup.rs`: `#[cfg(windows)] restart_task_later()` spawns it detached (DETACHED_PROCESS |
  CREATE_NEW_PROCESS_GROUP, null stdio, `raw_arg` so cmd sees `&` and `>` unquoted).
- `runner/src/main.rs`: the self_update branch calls it under `#[cfg(windows)]` before `exit(4)`.

## Checks

- `cargo test --locked` of the runner: 84 passed (new test `restart_line_waits_then_runs_the_task`, failed before).
- `tools/check-windows.sh` (mingw cross-check, all targets) passes; a wiring check (kireserver
  /tmp/kk-wr/chk/wincheck.sh) asserts the call sits under cfg(windows) right before exit(4).
- Not checked here: a real self-update on the Windows PC (the Windows thread does that after the deploy).

## Coder run log (OVMS on kireserver, one pipeline run per unit, up to 3 attempts)

| Unit                 | File        | Attempts | Result | Claude's correction                                                                     | Lesson   |
| -------------------- | ----------- | -------- | ------ | --------------------------------------------------------------------------------------- | -------- |
| A restart_line       | winsetup.rs | 1        | pass   | none                                                                                    |          |
| B restart_task_later | setup.rs    | 3        | fail   | wrote the 17-line function (Coder rewrote install_service, invented a schtasks /Change) | 291, 292 |
| C call before exit   | main.rs     | 3        | fail   | 3 lines: removed calls before the download, cfg(not(windows)) to cfg(windows), comment  | 293      |
| D version            | Cargo.toml  | 1        | pass   | none                                                                                    |          |
| E lock               | Cargo.lock  | 1        | pass   | none                                                                                    |          |

## Metrics

| Item               | Value                                                                                                                                               |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Coder (OVMS)       | 8 calls, 142 GPU s, 41,926 prompt and 3,905 output tokens (docs/qwen-log/win-restart.jsonl)                                                         |
| Claude             | unmeasured for this task alone: the thread's session totals at 01:31Z were 81 answers, 61,809 output, 36.4 M cache read (UX2 and this fix together) |
| Other local models | PR-Agent review: see the PR                                                                                                                         |
| Lessons            | 291-293 (situations 292, 293 in docs/qwen-situations/)                                                                                              |
