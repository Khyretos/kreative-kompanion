# kk-render: Kompanion ComfyUI renders on soucouyant

soucouyant (RX 9070 XT, 16 GB) is Kees's daily driver and the only machine for Kompanion's ComfyUI test renders
(the A770 on kireserver does the drafting). On 2026-10-19 three render runs killed the desktop. This runner is the
only way render jobs run there now.

## Why the desktop died

| When (UTC) | What ran | What happened |
|---|---|---|
| 03:14 | MV-Adapter, 6 views at 768, no cap | 15.5 GB on the card, Hyprland aborted |
| 04:38 | MV-Adapter and SDXL in one ComfyUI | MV memory stayed, 13.9 GB, desktop hung |
| 13:45 | SDXL pose sheet, 0.55 torch cap, 13 GB watcher | No VRAM line crossed (10-11 GB). The first image came out **all black** and the batch went on; the next job ended in a GPU page fault in Hyprland's context, a gfx ring reset, and Hyprland aborted. |

Lessons:
- Hyprland cannot survive **any** GPU reset, so the goal is "no GPU fault at all", not only "VRAM under X".
- A black or flat output is the warning sign: stop there, never run the next job.
- `expandable_segments:True` was back on in the 13:45 container (Kees turned it off on 2026-10-11: ROCm never gave
  that memory back), and gpu-guard only watched port 8188 with 1-minute heartbeats, so it was blind to the test port.
- `--reserve-vram` alone does not bound memory; the torch cap does, and ComfyUI must be told the same budget.

## What every job gets

1. **One at a time**: a lock file; a second `kk_render.py` refuses.
2. **Preflight** refuses to start while: any other container runs (CI, tests), a game runs (Funkin, Steam, Wine,
   gamescope...), the GPU averages over 30% busy for 5 s (the desktop needs it), more than 3.5 GB VRAM is in use,
   load is over 6, less than 8 GB RAM is free, or the junction is over 70 C. The Studio ComfyUI (`comfyui-rocm`)
   is the exception (Kees, 2026-10-24: do not refuse render jobs because of it): kk-render waits until its queue is
   empty (up to 30 min), `docker stop`s it for the job so two ComfyUIs never share the card, and starts it again at
   the end, whatever the job's result. The GPU checks run after it has stopped.
3. **A fresh container** `kk-render` on port 18190 (`comfyui-rocm:local`): torch capped at 50% of the card
   (`vram_cap.py`), `--reserve-vram 8.5` so ComfyUI plans inside that cap, `expandable_segments:False`, no pinned
   memory, bf16 VAE, 12 GB RAM cap, 8 CPUs. Only the SDXL model family is mounted (one family per container).
4. **A 4 Hz watchdog** `docker kill`s the container when the card total passes 12 GB, the junction reaches 95 C or
   memory 96 C, any amdgpu page fault, timeout or reset line appears in the kernel log, or another container starts.
5. **A hard timeout** per job (default 300 s).
6. **An output check**: a black or flat image fails the job and stops the batch. Nothing is retried automatically.
7. The container is **always stopped** at the end; the log prints peak VRAM and temperatures.

## Use

```sh
python3 -I ~/.local/lib/kk-render/kk_render.py check                 # preflight only
python3 -I ~/.local/lib/kk-render/kk_render.py run a.json b.json --timeout 300
```

Graphs are ComfyUI API format (what `/prompt` takes). Inputs go in `~/kk-render/in` (ComfyUI's `input`), outputs
appear in `~/kk-render/out/<filename_prefix>`. Job batches live in `comfy-jobs/<batch>/jobs.json` (graph, input
mapping, output path, timeout); copy inputs by that mapping.

Install on soucouyant: copy `kk_render.py` and `vram_cap.py` to `~/.local/lib/kk-render/`.

First results (2026-10-19): 512 px smoke test 8 s; 1024 px ControlNet check 18 s; nine pose renders at 832x1216 in
13-22 s each, card peak 9.2 GB, junction up to 84 C, no GPU faults.

## Not covered yet: 3D

MV-Adapter (multi-view) and TRELLIS.2 (image to 3D) are the jobs that blew past 15 GB. They need their own profile
(own container and models, lower cap, smallest settings first: few views, low resolution, VAE tiling) and a smallest
test before any real job.
