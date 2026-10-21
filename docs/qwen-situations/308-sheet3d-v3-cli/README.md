# 308: v3 template builder CLI (STU-SHEET3D, tools/pose-sheet/sheet_v3.py, Coder OVMS, 3 attempts)

- Job `job.json` (context: layout.py and tools/test_sheet_v3.py); no input file (new file); wrong output
  `wrong-sheet_v3.py`; corrected `corrected-sheet_v3.py`.
- Every function passed its tests on the first draft; only main() failed all 3 attempts. It read OUT_DIR as
  argv[3] for both commands (weapon has it at argv[2]: IndexError), saved the weapon lines-final.png at the base
  size instead of size x FINAL, and wrote a points.json nobody asked for (which also needs parts["points"]).
  The fix rounds kept the per-branch variables and never moved OUT_DIR into the branches.
