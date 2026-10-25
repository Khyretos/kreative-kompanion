# 3D-01 on soucouyant (2026-10-25)

Colour split found: TRELLIS.2's texture VAE decode on the RX 9070 XT gives colours of about +-200 (they should be
0..1) with one half of the model twice as noisy; the texture latent itself is fine (same statistics on both halves).
Replaying the dumped latent through the same decode on the CPU (`graphs/texdec-cpu.json`) gives 0..1 on both
halves. Fix: decode the texture on the CPU (`studio/comfy-nodes/kk_trellis.py`, KKVaeDecodeTextureCPU, ~33 s),
and load the shape VAE in fp32 per node (KKVAELoaderFP32) instead of starting ComfyUI with --fp32-vae.

Results (kk-render, profile trellis, --lowvram, torch cap 0.60, one fresh container per model):

| Character | Seconds | Card peak | Vertices | Faces | NaN |
|---|---|---|---|---|---|
| goblin | 120 | 11.7 GB | 100,617 | 199,051 | 0 |
| armour | 168 | 11.7 GB | 99,528 | 197,808 | 0 |
| dragon | 129 | 11.6 GB | 97,542 | 199,250 | 0 |
| hoodie | 135 | 11.7 GB | 97,282 | 198,893 | 0 |
| spear | 150 | 11.7 GB | 100,055 | 199,445 | 0 |
| goblin, production nodes (profile trellis-nodes, no global VAE flag) | 107 | 11.7 GB | 100,561 | 198,365 | 0 |

Two models in one container ran out of its 12 GB RAM cap (the CPU texture VAE stays loaded): one model per run.
The .glb files stay on soucouyant in ~/kk-render/out/3d01/ (previews here).
