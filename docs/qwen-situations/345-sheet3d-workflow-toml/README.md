# 345: two workflow.toml files (STU-SHEET3D, oc-sheet-v3 and weapon-sheet, Coder OVMS)

- Jobs `job.json`; context studio/workflows/oc-sheet/workflow.toml; wrong outputs `wrong-v3.toml`,
  `wrong-weapon.toml`; corrected `corrected-v3.toml`, `corrected-weapon.toml`.
- Both passed Claude's check (v3 first try, weapon second), but the review found: the v3 negative lost the
  minor-safety tags and {rating_neg} (asked: "the same as now without {helmet_neg} and without frame, border,
  panels"; it cut everything after the second removal), a kept comment was renamed (STU-R2 -> STU-C3), and the
  weapon [studio] table lost label, hint and sizes. Claude's check tested only what the job changed, not what it
  had to keep (lesson 66 again): a "keep" check compares every untouched line against the source.
