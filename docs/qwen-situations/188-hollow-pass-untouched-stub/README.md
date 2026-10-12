# Lesson 188: a check that cannot see an untouched stub passes hollow

Situation: job `gp-web-model` (M6-11, 2026-10-12): replace the body of `plan_with_model` (`todo!()`) in server/src/gameplan_web.rs. job.json is the second, fixed job (check: no `todo!()` left in the file, then the test binary compiles).
Wrong output: the first run's check was only `cargo test --no-run`; the Coder returned the file with the stub untouched (hollow-pass-gameplan_web.rs.txt) and the check passed in 1 attempt. Found because `grep todo!()` in the merged file still showed one.
Corrected: the check now fails while a `todo!()` is left. Rerun (OVMS, 4 attempts) still failed, see lesson 189.
Lesson (Claude's check): a compile-only check cannot tell "done" from "untouched"; every check must fail on the untouched tree.
