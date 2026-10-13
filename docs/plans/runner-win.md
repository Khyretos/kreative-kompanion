# RUNNER-WIN: the Kompanion runner on Windows (2026-10-13)

The runner (`kompanion-runner`) and the shared `machine-stats` crate now compile and run on Windows 10/11 as well as Linux.

## What changed

- `machine-stats`: `libc` is unix-only, `windows-sys` is Windows-only. `winsys.rs` reads memory (GlobalMemoryStatusEx), disk (GetDiskFreeSpaceExW), CPU time (GetSystemTimes), uptime (GetTickCount64); `winsample.rs` builds the same `Snapshot` as Linux (no GPUs, load 0, os "Windows"). Linux sampling is unchanged (`sample_unix`).
- Runner `platform.rs`: config folder (`%APPDATA%\kompanion-runner` or `~/.config/kompanion-runner`), `~` expansion, computer name, `config.toml` text with literal-string paths, `icacls` arguments (only the user may read the folder and the token), `taskkill` arguments, the system drive.
- Runner `winsetup.rs` + `setup::install_service` on Windows: a Scheduled Task `KompanionRunner` (logon trigger, restart after one minute, no time limit) registered with `schtasks /Create /XML` and started.
- Self-update: `replace_with_new` renames the running `.exe` to `.old`, puts the new one in place (rolls back on failure); `cleanup_old` deletes `.old` at the next start; the download name is `kompanion-runner.exe` on Windows.
- Shell tool runs `cmd /C` on Windows; service and package tools answer "not supported on Windows"; process trees are killed with `taskkill /T /F`.
- CI: `build.yml` job `runner-windows` cross-checks the runner for `x86_64-pc-windows-gnu` on soucouyant (also `sh tools/check-windows.sh`); `windows.yml` runs the tests natively and needs a Forgejo runner with the label `windows` (manual or push-to-main only until one exists).
- `docs/runner-install.md` has a Windows section.

## Not done / to verify on the Windows PC

- Nothing was run on Windows yet: the Windows build is compile-checked with mingw only. The PC must build, pair, run `install-service` and check that the task survives a reboot and that a self-update exit restarts it.
- The server does not serve `/download/kompanion-runner.exe` yet, so self-update on Windows answers with an error (nothing changes).
- No GPU stats, load average or game detection on Windows; the Windows `Sampler` takes `root` only for API compatibility.
- `proc::tests::test_timeout_kills_children` fails on kireserver on main as well (not caused by this change).

## Metrics

- Claude (this session so far, from 2026-10-13T12:34Z): answers 104, output 100,297, input 208, cache read 20,492,918, cache write 214,539.
- Coder (OVMS): 60 draft calls over 32 units, 36 job runs with a check: 14 passed (12 on the first attempt), 22 stopped after 3 attempts; 2,720 GPU s, 32,032 output / 518,348 prompt tokens. Many of the 22 were not the Coder's fault: two pipelines ran on one tree and a failed unit broke the build for every later unit (lesson 252). Claude then corrected the units that were truly stuck (winsys, platform, winsetup, selfupdate, system, sysinfo, grants, main) and filed lessons 246-253 with their situations in `docs/qwen-situations/`.
- Claude wrote `tools/check-windows.sh` before the first Coder job (a skipped step for that file); the CHECK_DIR change went through the Coder.
- Review/CI rounds, findings real/false and time worked vs waiting: unmeasured.
