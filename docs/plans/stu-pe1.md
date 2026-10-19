# STU-PE1: pose editor in Kreative Studio (queue thread 21, 2026-10-19)

Spec (queue-specs-2026-10-18.md): 3D skeleton of the base mesh (body, 21-point hands, head direction) with drag
handles, IK for arms and legs, finger curl sliders, mirror, joint limits; OpenPose + depth renders for ControlNet;
the same edits through a JSON op API (`set_joint`, `curl_finger`, `mirror`, `look_at`) so Claude and local models
fix a pose without the UI; poses saved to the library with provenance. Absorbs STU-P2; covers the puppet half of
Kees's STU-POSE (presets cowboy shot, action, sitting; the ControlNet "generate" half waits for ComfyUI).

## What was built (Kreative Studio branch `stu-pe1`, commit c1677d7, worktree kk-wt-pe1-studio)

- `public/js/pose/*.js`: vec, skeleton (base_mesh.py joints + 21-point hands), fk, ik (two-bone, pole), lookat,
  presets, ops, project (camera, OpenPose keypoints + JSON), raster (pixels, PNG), draw-openpose (port of
  draw_pose.py + hands), depth-parts + depth (ray-cast mannequin). One file runs in Node and in the browser.
- Ops: `set_joint {joint, bend, side, twist, add}`, `curl_finger {hand, finger, amount}`, `mirror {from: l|r|swap}`,
  `look_at {target | yaw, pitch}`, `ik {chain, target, pole}`, `move_root {pos}`, `reset {joints}`, `preset {name}`.
  Every value is clamped to the joint limits and the answer's `notes` say what was limited or out of reach.
- `lib/pose-api.js` (mounted in server.js): `GET /api/pose/info`, `POST /api/pose/ops`, `POST /api/pose/render`,
  `GET/POST /api/poses`, `GET/DELETE /api/poses/:id`, `GET /api/poses/:id/{openpose.png,depth.png,keypoints.json}`.
  Saved poses: `/data/poses/<user>/<id>/` with meta.json (pose, view, provenance: source, ops, parent, by, created).
- `tools/pose-cli.js`: `node tools/pose-cli.js --pose p.json --ops ops.json --out dir` for Claude and local models.
- Pose tab: canvas with handles (wrists/ankles IK, elbows/knees pole, nose look-at, pelvis root), tap a joint for
  bend/side/twist sliders, finger sliders + Fist/Open, mirror/flip/undo/reset, Turn/Tilt, Show (both/OpenPose/depth),
  Zoom (body, right hand, left hand), Ops (JSON), Save + My poses, OpenPose/Depth PNG downloads.

## Checks

- 14 new node test files (`test/pose-*.test.js`) and all 12 existing ones pass in the kreative-studio image.
- `test/e2e/pose.e2e.js` (13 sections: presets, IK drag, look/root drags, joint and finger sliders, ops JSON,
  save/load with provenance, flip, hand zoom and refit, 390 px both themes with contrast, no page errors) passes;
  ux1, ux2 and warnings e2e still pass.
- Done-when: the wrong hand fixed by a drag and an ops call is shown in the screenshots (sent 2026-10-19);
  the re-render with ComfyUI waits for Kees to unpause GPU work.

## Coder run log (OVMS, `docs/qwen-log/stu-pe1.jsonl`)

59 model calls, 4016 GPU seconds, 95,788 output and 642,999 prompt tokens, 30 jobs over 24 units.

| Unit | Attempts | Passed its check | Claude correction (lesson) |
|---|---|---|---|
| vec | 3 + 1 rerun | no | 2 slips (370, 371) |
| skeleton | 2 prompts, 3 | no | build section rewritten (372) |
| fk | 3 | no | 1 line (373) |
| ik | 3 + 3 repair | no | 6 lines (376) |
| lookat | 3 | blocked by Claude's test layout (389); code right | none |
| presets | 1 | yes (Node) | browser namespace (386) |
| ops | 3 + 3 repair | no | rewritten (377) |
| raster | 3 | no | PNG encoder (374), Claude's prompt notation (390) |
| project | 3 | no | 6 lines (375) |
| draw-openpose | 1 | yes | none |
| depth-parts | 3 | no | 6 spots (378) |
| depth | 3 | no | render rewritten (379) |
| api | 3 | no | requires + listing (380) |
| mount | 3 | blocked by the api unit (392) | 1 unused line (381) |
| cli | 3 | no | 8 lines (382) |
| stage | 1 + review repair | syntax only | 5 spots (383) |
| panel | 1 + review repair | syntax only | several (384) |
| library | 1 + review repair | grep only | 3 refs (385) |
| css, seg, html | 1 each | yes | none |
| stagezoom, panelzoom, htmlzoom, margin | 1 each | yes | outOfFrame loop (387) |

Units right without Claude: draw-openpose, lookat, css, seg, html, panelzoom, htmlzoom, margin (8 of 24).
Lessons 370-394 (370-387 Coder, 388-394 Claude/tooling), situations in `docs/qwen-situations/370-...387-...`.

## Claude tokens (claude-usage.py, this session up to the screenshots)

232 answers, 248,906 output, 464 input, 77,098,747 cache read, 460,058 cache write.

## Still open

- Kees's OK on the screenshots, then merge stu-pe1 into Studio main, `docker compose up -d --build` in
  remote-image-generation, live check, task state, cm_record.py, capability row.
- Image check (re-render a corrected hand with ControlNet openpose + depth) when ComfyUI is unpaused.
- pipeline.py lane crash on a cut-off first draft (369, 394) is still there.
