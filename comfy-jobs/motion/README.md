# STU-M1 and STU-M3: text to motion (2026-10-25)

## M1: which model

| Candidate | Code | Weights | Verdict |
|---|---|---|---|
| HY-Motion 1.0 / Lite (Tencent) | ? | tencent-hunyuan-community licence, `extra_gated_eu_disallowed: true` on Hugging Face | out: not licensed in the EU (same as Hunyuan3D) |
| MoMask (Guo et al., CVPR 2024) | MIT | trained on HumanML3D (research dataset built on AMASS; licence warning, Kees's rule: warn only) | picked |

MoMask runs on the CPU, so it needs no GPU slot: kireserver, `kk-momask:m1` image (`tools/momask.Dockerfile`, torch
2.4.1 CPU, numpy 1.23.5; `tools/momask-numpy-matplotlib.patch` fixes two old numpy/matplotlib calls), 6 cores, 6 GB cap.
20 prompts (`prompts20.txt`, lengths estimated by the model): 243 s for all 20 including loading and the mp4 previews;
the generation itself is a few seconds per clip. Output: BVH on the HumanML3D 22-joint skeleton, foot-contact IK
applied (`out/<n>.bvh`). The matplotlib previews came out blank; quality was judged on the retargeted character instead.

## M3: onto our characters

`tools/retarget.py` (Blender, CPU): imports a rigged 3D-01 character (STU-P1 .glb) and a MoMask BVH, turns the
source to the rig's facing (hip line), scales it by leg length, puts its hips on the pelvis, and per frame swings
each rig bone so the line to its child joint points where the source's does (parents first); the pelvis follows the
hips' path. The baked clip is added as an NLA track and exported. Checked: the goblin's wrist, elbow and head land
within 2-6 cm of the scaled source joints (`out/wave-strip.png`). Two earlier methods failed and are noted in the file.
