# Handoff: Kompanion ComfyUI work on soucouyant (2026-10-19 14:20Z)

State at handoff: no containers running, card at ~0.9 GB (desktop only), no amdgpu faults since the 13:45Z reboot.

## Rules (Kees, 13:47Z)
soucouyant runs only Kompanion's ComfyUI render work (the A770 on kireserver does drafting). No CI (Kees paused the
Forgejo runner in Forgejo; leave the `forgejo-runner` service alone), no Ollama/Coder, no builds. One job at a time,
smallest test first, stop on any sign of trouble, never retry blindly. Kees's daily driver: no surprise loads.

## The runner: kk-render
- Installed at `~/.local/lib/kk-render/` (`kk_render.py`, `vram_cap.py`, `kk_dump.py`); repo copy in
  `tools/kk-render/` with the crash write-up in `tools/kk-render/README.md`.
- `python3 -I ~/.local/lib/kk-render/kk_render.py check` runs the preflight only.
- `python3 -I ~/.local/lib/kk-render/kk_render.py run a.json b.json --profile sdxl|trellis --timeout 300` runs
  ComfyUI API graphs one after the other in a fresh `kk-render` container on port 18190, then stops it.
- Inputs: `~/kk-render/in` (= ComfyUI input). Outputs: `~/kk-render/out/<filename_prefix>`. Copy inputs by the
  batch's `jobs.json` mapping (file names in the repo differ from the names the graphs load).
- Guards: lock file; preflight refuses with other containers, games, a busy GPU, more than 3.5 GB VRAM, load over 6,
  less than 8 GB RAM; a 4 Hz watchdog `docker kill`s at 12 GB card total, junction 95 C / mem 96 C, any amdgpu
  fault/timeout/reset kernel line, or a new container; per-job timeout; black/flat PNG stops the batch.
- Profiles: `sdxl` = torch cap 0.50, `--reserve-vram 8.5`, bf16 VAE, SDXL family mounts.
  `trellis` = torch cap 0.60 (~9.5 GB), `--reserve-vram 6.8`, `--fp32-vae`, TRELLIS mounts + `kk_dump.py`.
  Never put two model families in one container. `expandable_segments` stays off (Kees, 2026-10-11).

## Done
- PE3 pose renders: round 1 (6 presets) commit 8747d57, round 2 (action, sitting, waving) 166d01b, both on
  `khyretos/pe3-renders` under `comfy-jobs/pe3-poses*/out/`. 13-22 s each, 9.2 GB card peak, no faults.
- Runner + write-up: a0daaf9 (`tools/kk-render/`).

## 3D-01 (TRELLIS.2 image to 3D): in progress
- Work folder `~/kk-3d01-work` (old notes `3d-01-state.md`, bug draft `comfyui-trellis2-colour-bug.md`, HOLD on
  posting upstream). Graph `g-up1024.json` (int8, upsample 1024, remesh drop_small_components 0). Crops copied to
  `~/kk-render/in/3d01/char-*.png`.
- Debug graph ready: `~/kk-render/jobs/3d01/dbg-goblin.json` = g-up1024 on the goblin + KKDump nodes that save the
  texture latent (node 12), shape latent (23), voxel colours (93) and shape subdivides (92) to
  `~/kk-render/out/dump/*.pt`.
- First run under the cap (14:17Z): **OOM inside the container in Trellis2UpsampleStage** (8.3 GB allocated, 9.55 GB
  allowed). Card peak 11.1 GB, desktop fine, nothing tripped. So int8 + upsample 1024 does not fit in 0.60; the old
  14 GB peak was not just slack. Do not raise the 12 GB kill line.
- Next for the budget: look at what the upsample stage holds (`shape_vae.upsample_shape` decodes the whole 512
  latent through the shape decoder at once); options are `--lowvram` / unloading the flow model before the upsample
  (it stays resident: 5.3 GB int8), a smaller structure, or chunking `upsample_shape`. Try `--lowvram` first.
- Colour bug (right on x<0, noise on x>0, clean plane): source read in `~/kk-3d01-work/src/` (copied from the image).
  Not found yet. Ruled out by reading: the sorted-array hashmap and neighbour maps in `trellis2/flexgemm.py`
  (int64 keys, chunking looks right), the z-up to y-up remap in VaeDecodeTextureTrellis (consistent with the mesh).
  Coords are sorted by (batch, x, y, z) after `unique()`, so "x>mid is wrong" = "the second half of the token
  list is wrong": suspect something index-based past ~N/2 (attention kernel on gfx1201 for long sequences in the
  texture flow model, or a chunk boundary). Plan once the job fits: run dbg-goblin, then on the CPU compare the
  texture latent statistics for x<mid vs x>mid (if the latent is already bad, it is the sampler/attention: retry
  the texture KSampler with ComfyUI `--use-split-cross-attention` or `--use-quad-cross-attention`); if the latent
  is fine, decode it with the texture VAE on the CPU in fp32 and compare with the GPU voxel colours.
- Done when the 5 crops give .glb with no NaN, no broken faces, correct colour on both halves (judge by the
  RenderMesh preview, not raster.py), seconds and VRAM peak per model; outputs (glb + preview) to
  `comfy-jobs/3d01/out/` on `khyretos/pe3-renders`.

## STU-SHEET3D (MV-Adapter SD2.1): not started
- Image `comfyui-mv:q19` (needs its own kk-render profile: mount `~/comfy-models-local/mvadapter` at `/mvm/mvadapter`
  and `~/kk-mv/diffusers-cache` at `/mvcache`, env `MV_HF_DIR=/mvcache MV_OFFLOAD=0 HF_HOME=/mvcache/hf`).
  Graph template in `~/kk-mv/mvrun21.py`; measured 4 views at 512 in 9-12 s, 6.9 GB card.
- Run MV in its own container, stop it, then the SDXL refine with `--profile sdxl`.
- Test set `~/kk-sheet3d-work` (best so far out/q20/knight-sheet.png). Owed fixes: black head grid (try 1024 with
  tiled VAE), head box 0.24, dot grid in the back view, armour drift at denoise 0.4 (try 0.3). Sheet code on
  `khyretos/stu-sheet3d` (layout.py, sheet_v3.py, cut.py).

## Reporting
Results and commit hashes go to the project coordinator as notes; thread replies only for results, blockers or
one-word decisions for Kees.
