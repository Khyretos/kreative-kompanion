# 307: anatomy check by layout cells (STU-SHEET3D, tools/anatomy/check.py, Coder OVMS, patch mode)

- Jobs `job.json` (two patch jobs: cells + judge/detect, then parse_query/check/main); input `input-check.py`;
  wrong output `wrong-check.py` (after both jobs); corrected `corrected-check.py`.
- Job 1 (3 attempts, failed): judge() judged faces inside the loop over face boxes, so a cell with no face never
  got "no face"; the old per-cell loop stayed below the new `return` as dead code, and two extra copies
  (judge_v3_old, judge_v3) were added. detect() looped models first (wrong call order) and dropped the y offset
  (`bx, by, bw, bh = b`, then by not shifted). The file grew from 225 to 480 lines.
- Job 2 (parse_query, check, handler, CLI) was right except the per-image CLI call, which did not pass
  args.layout. Its check passed on attempt 2 only because Claude had already rewritten judge/detect in the same
  file while the fix round of job 2 was running (lesson 307, second half).
