# STU-PE2: animation from poses in Kreative Studio (queue thread 22, 2026-10-19)

Spec (queue-specs-2026-10-18.md): timeline of key poses, interpolation (slerp + IK), ease curves, loop, preview;
export BVH and .glb; export an OpenPose frame sequence for ControlNet video or sprite sheets (M6-07).
Done when: a walk cycle from 4 corrected key poses exports to BVH/.glb, opens in Blender, and renders a sprite strip.

## What was built (Kreative Studio branch `stu-pe2` on top of `stu-pe1`, commit ebbed79, worktree kk-wt-pe2-studio)

- `public/js/pose/quat.js` (quaternions), `anim.js` (clips: keys, eases linear/in/out/in-out/hold, loop wrap,
  slerp per joint, root lerp, IK targets on both neighbouring keys solved after the slerp), `clips.js` (4-key walk).
- `bvh.js` (BVH, Y-up, ZXY channels, columns in hierarchy order), `glb.js` (container), `mannequin.js` (one box
  per bone, rigid weights), `gltf.js` (skinned mannequin + one animation), `frames.js` (OpenPose keypoints per frame
  in one fixed camera, sprite strip in a row or a grid).
- `lib/anim-api.js`: `GET /api/anim/info`, `POST /api/anim/sample`, `POST /api/anim/export` (bvh, glb, strip,
  frames), `GET/POST /api/anims`, `GET/DELETE /api/anims/:id`, `GET /api/anims/:id/{clip.bvh,clip.glb,strip.png,frames.json}`;
  saved clips in `/data/anims/<user>/<id>/` with provenance.
- `tools/anim-cli.js`: clips from saved poses (`--poses a.json,b.json`) or a clip file, `--out x.bvh|.glb|.png|.json`, `--sample t`.
- Pose tab, Animation section: Walk cycle example, timeline with numbered key markers, scrub, play/pause, add/update
  and remove key, ease per key, fps, length, loop, exports (BVH, .glb, sprite strip, OpenPose frames, hands option),
  save clip and My clips (strip preview, load, delete).

## Checks

- 10 new node test files, 122 node tests pass in the kreative-studio image (1 GB cap).
- `test/e2e/anim.e2e.js` (walk, scrub, add/remove, ease, play, export, settings, library, 390 px both themes with
  contrast, no page errors) passes; pose, ux1, ux2 and warnings e2e still pass. The e2e container now runs with
  `--memory 2g --cpus 2`.
- Blender 4.3 (blender-mcp image, capped): `test/blender/expect.js` + `check_anim.py` import the walk's BVH and .glb;
  every bone head matches the Studio FK at all 24 frames (worst 0.0005 mm BVH, 0.0002 mm glb), and the skinned
  mannequin renders as an 8-frame side-view strip.

## Coder run log (OVMS, `docs/qwen-log/stu-pe2.jsonl`)

30 model calls, 2427 GPU seconds, 55,414 output and 444,352 prompt tokens, 18 jobs over 16 units.

| Unit | Attempts | Passed its check | Claude correction (lesson) |
|---|---|---|---|
| quat | 1 | yes | none |
| anim | 3 | no (Claude's test was wrong) | test fixed, code kept (395) |
| glb | 3 | no | 8 lines (396) |
| bvh | 3 | no | about 30 lines (397); column order (398), brace lines (400) |
| mannequin | 1 | yes (weak test) | 1 line (401) |
| gltf | 3 | no | toGLTF rewritten (399) |
| frames | 1 | yes | none |
| api | 1 | yes | none |
| clips | 1 | yes | none |
| cli | 3 | no | about 25 lines (402) |
| stage, css, mount | 1 each | yes | none |
| html | 1 | id count only | 2 closing tags, script order (405) |
| animui | 1 + 3 repair | syntax only; repairs worse | 3-line lazy stage lookup on the first draft (403) |
| libui | 1 + 1 review repair | yes | none (404) |

Units right without Claude: quat, mannequin (after 401), frames, api, clips, stage, css, mount, libui (after review).
Lessons 395-406 (395, 398, 400, 406 Claude; the rest Coder), situations in `docs/qwen-situations/395-...406-...`.

## Still open

- Kees's OK on the screenshots (Animation section, 390 and 1300 px, both themes), then merge stu-pe1 and stu-pe2
  into Studio main, `docker compose up -d --build` in remote-image-generation, live check, task state, cm_record.py.
- ControlNet video/sprite render from the OpenPose frames waits for ComfyUI (paused).
