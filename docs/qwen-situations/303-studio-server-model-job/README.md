# 303: 3D refine as a model job (3D-01, Kreative Studio server.js, Coder OVMS, 3 attempts, twice)

- Job `job.json` gave both new lines verbatim and quoted both anchors; input `input-server.js` (1,134 lines).
- Every attempt quoted SEARCH text that is not in the file (an invented `trimJobs(); res.json(...)` block), and the
  last one added an unrelated branch to the image route that serves a .glb as the preview. `wrong-server.js` is the
  clean second run's output; `corrected-server.js` is Claude's two-line change.
