# 306: v2 sheet keypoints without Blender (STU-SHEET3D, tools/pose-sheet/v2_points.py, Coder OVMS, 3 attempts)

- Job `job.json` (context: tools/pose-sheet/base_mesh.py and tools/test_sheet_layout.py); no input file (new file);
  wrong output `wrong-v2_points.py`; corrected `corrected-v2_points.py`.
- The spec spelled the hidden-face rule out step by step (subtract the head, rotate, flatten, dot with (0, -1),
  None when below -0.2). The draft dropped the subtraction and the None, inverted the test and appended a
  projection of the direction vector instead. It also did `rot_j[0] += x_offset` on the tuple its own rotate()
  returns; both fix rounds patched one unrelated line and the TypeError stayed.
