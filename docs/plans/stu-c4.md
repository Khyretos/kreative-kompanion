# STU-C4: anatomy check and automatic retry, results (2026-10-11)

PR #163 (check + retry) and #165 (GPU guard, ComfyUI memory fix). Deployed as main 60ccf4b with
`COMPOSE_PROFILES=anatomy` and `anatomy_url = "http://anatomy:8000"`.

## 20-sheet before/after

oc-sheet, 2 characters x 10 seeds (101-110), one at a time through the API as the test user.
"Before" is the first try alone (no re-roll), "after" is the kept result with up to 3 re-rolls.

| Character | Sheets | First try passes | After re-rolls |
|---|---:|---:|---:|
| A: woman knight, silver hair, blue cloak, armour | 10 | 2 | 6 |
| B: man, short black hair, red hoodie | 10 | 5 | 10 |
| Total | 20 | 7 (35 %) | 16 (80 %) |

50 renders in all (2.5 per sheet), 145 s average, 183 s max. The 4 sheets still failing (all A): front
view shows the back of the head (2), three faces in the left view (2, one also three hands). Spot checks
of kept and failed sheets agreed with the checker.

## GPU during the run (amdgpu sysfs every second)

Peak 14.7 GB VRAM used (card incl. desktop), junction 87 C, memory 94 C. No ring timeouts or GPU
resets since the 17:11 boot, no out-of-memory in ComfyUI, the guard cancelled nothing.

## Root cause of the hangs and stuck sheets

- Both resets (16:41, 16:47) were desktop GPU work (Claude app, Sunshine) timing out while ComfyUI held
  the card nearly full. ComfyUI ran `--highvram --reserve-vram 1.5`.
- With `expandable_segments` on ROCm, memory torch freed never returned to the card: after a sheet torch
  reported 240 MB reserved and KFD 0.7 GB for ComfyUI, yet amdgpu showed 11.2 GB used until a restart.
  One sheet in that state came out as pure noise.
- Fix (soucouyant `~/.config/comfyui-rocm/compose.override.yml`, backup `.bak-2026-10-11`): no
  `--highvram`, `--reserve-vram 3`, `PYTORCH_CUDA_ALLOC_CONF=expandable_segments:False` (the base file sets
  it to True). After two back-to-back sheets `/free` takes the card from 7.3 GB to 1.1 GB.
- Cost: a sheet try takes 136-183 s instead of ~104 s because models reload.

## Guard

`tools/gpu-guard` runs as a user unit on soucouyant (`sh tools/gpu-guard/install.sh`). It clears ComfyUI's
queue and calls `/interrupt` when free VRAM stays under 1 GB for 6 s or junction/memory reaches 100 C for
4 s; never kills anything. Studio then shows "stopped by the GPU guard or a cancel".
`journalctl --user -u gpu-guard` on soucouyant.

## Scheduler figure

oc-sheet keeps `vram_mb = 11000`: the measured 13.4 GB peak plus 10 % would exceed what the 16 GB card
offers next to the ~2.3 GB desktop and the 2 GB headroom, so the sheet would never start; ComfyUI
offloads to fit.
