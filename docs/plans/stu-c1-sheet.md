# STU-C1: character sheet v2 (2026-10-09)

Goal: one OC sheet, 3000+ px wide, with four standing views (front, left side, back, right side), each
facing the right way. Covers M6-S02 (OpenPose) and M6-S10 (the friends' examples).

## What the workflow does now

`studio/workflows/oc-sheet` (v2; the old graph is `oc-sheet-classic`):

1. Base 1344x672, novaAnimeXL + character_Sheet_XL 0.6 + Marvel Rivals style 0.8 (unchanged LoRAs).
2. ControlNet union SDXL promax (xinsir, Apache-2.0, `xinsir-union-sdxl-promax.safetensors`), applied twice
   before the base sampler: OpenPose (strength 0.7, end 0.8) and depth (0.4, end 0.6).
3. Pose and depth come from a Blender base mesh: `tools/pose-sheet/base_mesh.py` builds an 8-head mannequin,
   places four copies rotated 0/-90/180/90 degrees, renders depth with an orthographic camera and writes the
   COCO-18 keypoints (face points turned away from the camera are hidden, so the back view has no nose or eyes).
   `draw_pose.py` (Coder) draws the OpenPose image. Both PNGs ship in the workflow folder; `[images]` in
   workflow.toml uploads them to ComfyUI before each run.
4. Hires at 2016x1008 (denoise 0.35, without ControlNet), face and hand detailers, then 4x-AnimeSharp and a
   lanczos scale to 3024x1512.

Regenerate the templates: `docker run --rm --entrypoint /opt/blender/blender -v $PWD/tools/pose-sheet:/w -v <out>:/o blender-mcp -b -P /w/base_mesh.py -- /o 1344 672`
(the shipped files were rendered at 1536x768; ControlNet scales them to the latent), then
`python3 tools/pose-sheet/draw_pose.py <out>/keypoints.json pose.png`.

## Findings (Loquendo, soucouyant RX 9070 XT)

| Variant | Result |
|---|---|
| classic graph at 1536x768 | 3 of 4 figures seen from behind, plus extra heads |
| OpenPose only (0.7/0.8) | ignored on one seed (6 figures + heads); on another the left profile turned 3/4 back and the back view looked over its shoulder |
| OpenPose + depth | four correct views on 4 of 4 seeds (300, 303, 304, 305) |
| + regional prompts per quarter | worse: each quarter became its own framed poster |

- Depth is what fixes the facing; OpenPose alone does not. Depth from a thin mannequin did not make bulky outfits thin.
- VRAM: 1536x768 with both controls ran out of memory on the 9070 after a fresh load (CLIP reloaded); runs with
  cached text encodings passed, which hid it. 1344x672 fits: 136-170 s per sheet, torch peak about 13 GB.
- `PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True` made it worse on ROCm (about 14 GB stayed held outside
  torch's own counters after /free); reverted.
- One seed (304) painted a purple starry background; "gradient background, starry sky, frame, border, panels" is
  now in the negative.
- The monocle and moustache are not in every view; raise them in the prompt when they matter.

Comparisons: `/media/Generated/images/kompanion/stu-c1/compare/`.

## Not done here

- Kreative Studio's OC sheet already runs through Kompanion's `oc-sheet` (KS-01), so it gets v2; its local
  fallback graph (`lib/graphs.js`, used only with KOMPANION_JOBS=off) and studio-gen.py keep the classic graph.
- IP-Adapter applies to the whole sheet, not only the front view (C2/C3 take identity further).
- 4x-AnimeSharp is CC BY-NC-SA 4.0 (non-commercial); it shows as a licence warning.
