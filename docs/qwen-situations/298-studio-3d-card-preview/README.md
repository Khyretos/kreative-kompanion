# 298: 3D card preview, branch rewritten the wrong way (3D-01, Coder OVMS, 3 attempts)

- Job: `job.json` (patch on web/src/views/studio.ts; check: tsc and tests/studio3d.spec.ts, studiomedia, studiocheck).
- Input: `input-studio.ts`. Wrong output after attempt 3: `wrong-studio.ts`. Corrected by Claude: `corrected-studio.ts`.
- Attempt 1 quoted a SEARCH text that is not in the file (patch refused twice).
- Repairs kept giving every file a View in 3D button, wrapped each pair in an `<li>` inside a `<div>` and never added
  the preview `<img>` for r.files[1], so the Select/Delete overlay still covered the button.
- Rerun: same job.json against input-studio.ts; a one-shot pass shows one preview img and one button for r.files[0].
