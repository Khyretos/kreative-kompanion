# 304: optional slider in the refine form (3D-01, Kreative Studio public/js/refine.js, Coder OVMS, 3 attempts)

- Job `job.json`; input `input-refine.js`; wrong output `wrong-refine.js`; corrected `corrected-refine.js`.
- It hid the slider group with an early `return` inside pick(): the group was never shown again for the next step
  and the `sync()` call after it (the Make button state) was skipped. Two of its checks timed out on Claude's
  e2e harness (lesson 305), the last one ran and failed on the hidden slider.
