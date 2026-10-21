# 344: layout setting for the anatomy check (STU-SHEET3D, server/src/studio/anatomy.rs, Coder OVMS, patch mode)

- Jobs `job.json` (R1 workflow.rs field: right on the first try; R2 anatomy.rs: 3 attempts, failed); input
  `input-anatomy.rs`; wrong output `wrong-anatomy.rs`; corrected `corrected-anatomy.rs`.
- R2 did steps 1 and 2 (parameter, query building) exactly as given but skipped step 3 (the two call sites).
  The fix rounds then answered the E0061 errors at the call sites by changing reseed()'s signature (an unrelated
  function, now indexing p.also by position) and pasted a second copy of best() and summary().
