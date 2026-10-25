# STU-M1..M4: motion from text (queue 24, 2026-10-25)

- M1 pick: MoMask (code MIT; weights trained on HumanML3D, a research dataset: licence warning per Kees's rule).
  HY-Motion 1.0 is out: its licence (tencent-hunyuan-community) disallows the EU. Runs on the CPU, so no GPU slot.
  20 test prompts and notes: comfy-jobs/motion on khyretos/pe3-renders.
- M2 (this branch): `tools/motion` service (`POST /motion {prompt, seconds, seed}` -> 20 fps joints + BVH), compose
  service `motion` (profile `motion`). Measured on kireserver: models load in 12.7 s, a 3 s clip in 3.8 s, a 4.4 s
  clip in 5.2 s. Kreative Studio's Pose tab gets "Motion from text" (Studio branch stu-m2: /api/motion, OpenPose frame
  preview, BVH and frame downloads).
- M3: `retarget.py` (Blender) puts a generated BVH onto a rigged STU-P1 character (comfy-jobs/motion).
- M4 test: 8 frames of a generated jump through SDXL ControlNet (comfy-jobs/m4); figures follow the poses, some
  frames come out from behind and the outfit drifts (next: IP-Adapter from the OC sheet, depth frames).

Deploy: models in ~/Docker/Services/ai/motion (checkpoints/ 194 MB, clip/ViT-B-32.pt 338 MB), `KOMPANION_MOTION_MODELS`
pointing there, `--profile motion`; Studio `MOTION_URL=http://motion:8000` in its compose override.
