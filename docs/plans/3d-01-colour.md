# 3D-01: the colour split fixed (queue 24, 2026-10-25)

Before: every TRELLIS.2 model made on soucouyant had correct colours on one half (x < 0) and coloured noise on the
other, a clean plane split; the job also ran out of memory under the render cap at upsample 1024.

Cause: the texture VAE decode on the RX 9070 XT (gfx1201). Dumps of one goblin run showed a healthy texture latent
(same statistics on both halves) but decoded colours of about +-200 where 0..1 is expected, with the noisy half
spreading twice as wide. The same latent through the same decode on the CPU gave 0..1 on both halves.

Fix: `studio/comfy-nodes/kk_trellis.py` (two nodes, mounted read-only into soucouyant's ComfyUI custom_nodes):
`KKVaeDecodeTextureCPU` (the texture decode on the CPU, about 33 s) and `KKVAELoaderFP32` (the shape VAE in fp32,
so ComfyUI no longer needs a global `--fp32-vae` that would double every SDXL decode). The image-to-3d graph uses both.
Memory: ComfyUI `--lowvram` keeps the upsample stage inside a 0.60 torch cap (11.7 GB card peak).

Results on five library characters (kk-render on soucouyant, previews in comfy-jobs/3d01 on khyretos/pe3-renders):
107-168 s each, 11.6-11.7 GB card peak, 97-101 k vertices, 198-199 k faces, 0 NaN, correct colours on both halves.
Two models in one container ran out of its 12 GB RAM cap (the CPU texture VAE stays loaded), so one model per run.

Deploy (after Kees's look-OK): mount `studio/comfy-nodes/kk_trellis.py` at `/app/custom_nodes/kk_trellis.py` in
soucouyant's comfyui-rocm and restart only that container; then merge this PR. The upstream bug report draft
(comfyui-trellis2-colour-bug.md) can now name the texture decode on gfx1201; Kees decides whether to post it.
