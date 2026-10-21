# 297: openModelUrl, invented anchor and dropped casts (3D-01, Coder OVMS, 3 attempts)

- Job: `job.json` (patch mode on web/src/viewers/index.ts; check: exact signature grep, tsc, tests/viewers.spec.ts).
- Input: `input-index.ts`. Wrong output after attempt 3: `wrong-index.ts`. Corrected by Claude: `corrected-index.ts`.
- Attempt 1 quoted a SEARCH anchor that does not exist (a `viewerKind(a: AssetDetail): "model" | ...` signature).
- Attempts 2-3 put the function before openViewer instead of after it, used ext "fbx" for a .glb and left out the
  `as unknown as AssetDetail` / `as unknown as AssetsApi` casts the prompt named, so tsc refused the object literal.
- Rerun: same job.json against input-index.ts; a one-shot pass compiles and keeps tests/viewers.spec.ts green.
