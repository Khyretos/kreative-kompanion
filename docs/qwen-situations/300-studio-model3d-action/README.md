# 300: model3d step with an invented type (3D-01, Kreative Studio lib/refine.js, Coder OVMS, 3 attempts)

- Job `job.json`; input `input-refine.js`; wrong output `wrong-refine.js`; corrected `corrected-refine.js`.
- The prompt named the Kompanion type `image-to-3d` twice; every attempt wrote `type: 'refine-3d'` (the pattern of
  the neighbouring refine-* entries) and the test failed on that one value. Claude changed the one word.
