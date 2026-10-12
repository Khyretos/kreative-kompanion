# Lesson 200: a check that fails on the environment makes the repair loop invent code

Situation: RUNNER-SYNC units rsa1 (OVMS, Cargo.toml) and rsa2 (soucouyant, main.rs). Both first drafts were right (`sha2 = "0.10"`, `mod selfupdate;`). Claude's check scripts ran `cargo` over ssh without a PATH, so every check failed with `cargo: not found`. Two repair rounds each then invented code: `edition = "2021"` in Cargo.toml and a 10-line block in main.rs that sets PATH for "potential self-update operations" (wrong-output.diff).
Fix: Claude's mistake, not the Coder's. Reverted both files, put `export PATH=$HOME/.cargo/bin:$PATH` in the scripts and reran; both passed on attempt 1. The voided logs are kept as `*-void-check-bug.log.jsonl` in runner-sync-unit-runs/.
Rule: run every check script once by hand over ssh before queueing, against the untouched tree (must fail for the right reason) and against a hand-made correct edit (must pass). Same family: 202.
