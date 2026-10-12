# Lesson 193: the Coder adds its own test module next to the file's existing one

Situation: GAMEPLAN-LIVE (2026-10-12), job `gp-fill-titles` (jobs.json): replace the body of `fill_titles` (a stub) in server/src/gameplan.rs. The tests live in gameplan_tests.rs (`#[path]` module `tests`, in the context).
Wrong output (OVMS, 4 attempts, 5 calls; wrong-*.rs.txt): the function itself was right, but the draft also added an inline `mod tests { ... }` with its own test, a second module named `tests` (E0428 duplicate definition) whose test used a non-`mut` plan (E0596). Repair rounds quoted the errors and did not remove the block.
Corrected: Claude deleted the added block (24 lines); the real tests pass. Also Claude's mistake: the 60-character test used "word " x 30, whose 60th character is a space that the spec's trim removes; the test now uses 100 `a`.
Lesson: say "do not add tests or modules; the tests are in X" in jobs for a file that has a `mod tests`. Rerun after model refinement: run jobs.json (gp-fill-titles) and check no module is added.
