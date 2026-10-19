# STU-PE3: pose check in Kreative Studio (queues 23-24, 2026-10-19)

Spec (queue-specs-2026-10-18.md): model-assisted pose check. DWPose (Apache-2.0) reads the pose in a generated
picture, the check compares it with the target pose bone by bone, and the fix comes back as PE1 ops for a local
model or Claude. Kees (2026-10-19) added colour-coded body parts so the generator knows which part goes where.

## What was built

Kreative Studio branch `stu-pe3` (worktree kk-wt-pe3-studio, stacked on stu-pe2; local only, never pushed):

- `public/js/pose/posecheck.js`: `fromOpenPose` (also ComfyUI's POSE_KEYPOINT list), `swapSides`, `compare` (2D
  angle per bone for 10 body bones, `ok`/`wrong`/`missing`/`skip`, tolerance 20 degrees, a left/right swap only
  when it measures as many bones).
- `public/js/pose/posefit.js`: `fit` (per wrong bone a 15 degree grid over the joint's bend/side and the parent's
  twist, then refinement down to 0.1 degree; the child bone breaks ties) returns set_joint ops; `regions` gives
  repaint boxes.
- `public/js/pose/partmap.js`: the part-colour map, 17 body parts with one flat colour each, nearest shape wins.
  Pose tab Show = Parts, Parts PNG, `parts.png` in saved poses, `/api/pose/render` and `tools/pose-cli.js`.
- `lib/posecheck-api.js`: `GET /api/pose/check/info`, `POST /api/pose/check` (OpenPose JSON, or a picture sent to
  DWPose at POSE_DETECT_URL; its own 25 MB body limit). `tools/posecheck-cli.js` for scripts and agents (exit 0
  pass, 2 off, 1 error; `--out` writes the fitted pose).
- Pose tab "Check a picture": file (picture or JSON), allowed difference, per-bone list, overlay of the target
  skeleton on the picture with boxes, Match the picture (applies the ops, undoable), Repaint mask PNG.
- Fixed on the way: PE1's OpenPose/Depth PNG buttons never worked (undefined `stage`); e2e containers now end
  node themselves (`timeout 540` inside run.sh).

Kompanion (this branch): `tools/anatomy/dwpose.py` (YOLOX person boxes + DW-LL ucoco 384 whole-body keypoints on
onnxruntime CPU, OpenPose JSON out) and `POST /pose` on the anatomy service. Models are mounted from
KOMPANION_POSE_MODELS (yolox_l.onnx and dw-ll_ucoco_384.onnx from yzd-v/DWPose, Apache-2.0, sha256 7860ae79...
and 724f4ff2...). The Studio reaches it at `http://anatomy:8000` on the kreative-kompanion_default network.

## Results on real renders

Nine SDXL renders (novaAnimeXL + xinsir union ControlNet, OpenPose + depth, on soucouyant; jobs in
comfy-jobs/pe3-poses*, branch khyretos/pe3-renders). DWPose takes 0.6-0.8 s per 832x1216 picture on kireserver's
CPU.

| Render                                        | Check | Worst bone                       |
| --------------------------------------------- | ----- | -------------------------------- |
| t_pose, cowboy_shot, fists                    | pass  | under 7 degrees everywhere       |
| action (round 1 and 2)                        | pass  | right upper arm 17.8 (round 1)   |
| sitting (round 1, came out standing)          | off   | thighs 176, forearms 107-108     |
| waving (round 1, pointing sideways)           | off   | right forearm 92.9               |
| sitting (round 2, 35 degree turn, on a stool) | off   | thighs 21.5-22.4 (foreshortened) |
| waving (round 2)                              | pass  | right forearm 6.2                |

The check flags exactly the three renders a person calls wrong.

## Checks

- Studio: 160 node tests (6 new files) and every e2e (check, pose, anim, ux1, ux2, warnings) pass in the capped
  containers; screenshots at 1280 and 390 px, light and dark, for Kees's approval before merge.
- Kompanion: `python3 tools/test_dwpose.py` (11 tests with fake ONNX sessions, one end-to-end chain test).

## Coder

Units: check, fit, partmap, api, cli, dwpose, checkui (log docs/qwen-log/stu-pe3.jsonl). First try: partmap, api,
cli. Claude rewrote check, fit and checkui and fixed dwpose (lessons 407-418, situations 407, 408, 414, 416).
