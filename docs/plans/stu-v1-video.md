# STU-V1: why Studio video looked bad, and the new defaults (2026-10-08)

Covers M6-S05 (video rerun with real motion, colour drift).

## Test set

Wan 2.2 TI2V-5B Q8 GGUF on soucouyant (RX 9070 XT, ComfyUI `--highvram --reserve-vram 1.5`), 73 frames
(3 s at 24 fps), 20 steps, cfg 5, shift 8, one change at a time, fixed seeds:

| Clip | Kind | Seed | Prompt |
|---|---|---|---|
| fox | text to video | 1001 | a fox running through a snowy forest, camera following |
| rain | from a start picture (VN scene, woman at a rainy window) | 2002 | she turns her head toward the window, rain runs down the glass, her hair moves slightly, camera still |
| walk | from a start picture (scene, full-body figure in a hall) | 3003 | the man turns his head to look back over his shoulder, then walks two steps toward the camera, camera still |

Measured with `tools/video-metrics.py` (per-frame ffmpeg signalstats: brightness, saturation and contrast change
from the first 8 to the last 8 frames, colour cast, edge density, flicker = brightness jitter against the
neighbouring frames) and judged on first/middle/last frame sheets from `tools/video-compare.py`.

## Cause

The umt5 text encoder (fp8, about 6 GB) stayed on the 16 GB card next to the video model (`--highvram`):

- text to video ran out of VRAM in the tiled VAE decode every time, even at 512x288x25;
- from a start picture, ComfyUI partly unloaded the GGUF model to make room and every frame after the first came
  out as coloured noise. The earlier runs (M6-S05) showed the milder form: frame 1 fine, later frames blown out
  to high-contrast orange.

`CLIPLoader` with `device: "cpu"` keeps the encoder in system RAM: no OOM, no noise, colours held. Encoding on
the CPU adds a few seconds; a 3 s 832x480 clip takes about 110 s, 1280x704 about 235 s.

## A/B on top of the CPU encoder (832x480; drift and flicker, lower is better)

| Change | fox flicker | rain y / flicker | walk y / flicker | Verdict |
|---|---|---|---|---|
| base (uni_pc, temporal 16/4) | 1.76 | -4.7% / 0.45 | +1.5% / 0.44 | |
| steps 30 | 2.29 | -3.3% / 0.45 | +1.4% / 0.54 | slower, no gain |
| cfg 4 | 1.81 | -4.3% / 0.46 | +1.0% / 0.43 | same |
| shift 5 | 2.52 | -7.0% / 0.47 | -0.8% / 0.39 | fox contrast -91%: worse |
| euler | 1.08 | -2.2% / 0.48 | +1.5% / 0.40 | natural colours, sharper fox |
| temporal tiles 32/8 | 0.91 | -3.6% / 0.17 | +1.8% / 0.29 | less flicker everywhere |
| tiles 512/64 + 32/8, plain decode | - | - | - | out of VRAM |
| 16 fps (49 frames) | 3.46 | +11.1% / 2.02 | +5.2% / 0.48 | worse |
| euler + 32/8 | 0.53 | -1.2% / 0.17 | +1.8% / 0.18 | **new default** |

At 1280x704 (the Studio default) the old sampler drifted blue and blurred the start picture (rain: brightness
+21.6%, saturation +35%, flicker 2.63); euler + 32/8 kept it (-1.7%, +4.4%, 0.12). Fox flicker 0.75 -> 0.27,
walk 1.26 -> 0.12.

Encode: ComfyUI's h264 (about 3.5 Mbit/s at 832x480) scores SSIM 0.966 against the lossless frames, x264 CRF 16
slow 0.972 at 27% more bytes: not worth a re-encode step. No frame interpolation (16 fps was worse already).

## New defaults

Video graph in Kompanion (`studio/workflows/video/graph.json`), Kreative Studio (`lib/video.js`) and
`Services/ai/gpu-share/studio-gen.py`, kept identical (nodes 2, 4, 8, 9, 10 diffed): text encoder on the CPU,
sampler euler/simple, VAEDecodeTiled 256/32 with temporal 32/8. Steps 20, cfg 5, shift 8 stay.

Clips for Kees: `/media/Generated/video/stu-v1-ab/` (README inside).

## Not done

- fp16 text encoder: not on soucouyant (11 GB download); with the encoder on the CPU it no longer competes for VRAM.
- The A770 is still not allowed for video (`machines`): its XPU crash came from the fp8 encoder on the GPU, which
  this change moves off the card; measure it there before adding kireserver.
- The 5B model does not follow "look back over his shoulder": the figure walks forward. Prompt adherence, not quality.
