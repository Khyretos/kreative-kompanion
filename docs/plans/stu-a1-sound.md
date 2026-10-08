# STU-A1: why generated sound was choppy (2026-10-08)

Measured with `tools/audio-metrics.sh` (peak, RMS, loudness, flat factor = clipped runs, click = biggest
sample jump, gaps = silences under -50 dB over 30 ms inside the clip). Target: no gaps, peak -6..-1 dB.
All runs on soucouyant's RX 9070 XT. The clips are in `/media/Generated/audio/stu-a1-ab/` for listening.

## Causes and fixes

| Problem | Cause | Fix |
|---|---|---|
| SFX quiet and "broken up" (peak -19 dB, RMS -44 dB, 12-13 gaps) | MOSS returns a quiet clip; at -44 dB RMS the softer parts fall under -50 dB and read (and sound) like holes | `post.finish`: 10 ms fade in/out, peak to -3 dBFS. Same clip: 12 gaps -> 0, RMS -44 -> -28 dB |
| Every SFX of one prompt sounded the same | `sfx/server.py` called `torch.manual_seed`, but the MOSS pipeline seeds its own noise (`seed=0` default) | the seed now goes into the pipeline call (random when none is given) |
| Music clipped (peak 0.0 dB, flat factor 3.7-15.5, clicks up to +3 dB) | HeartMuLa's codec returns peaks around 2x full scale, and heartlib wrote them as 16-bit PCM | `heartmula/server.py` replaces heartlib's `postprocess`: fade and peak -3 dBFS before writing. flat 15.5 -> 0 |
| Music jobs failed with HTTP 500 on soucouyant | torch.compile wanted a C compiler for Triton | `TORCHDYNAMO_DISABLE=1` in the soucouyant compose file |
| 3 of the last 7 SFX jobs failed ("error sending request") | Kompanion asks the runner to `docker start` a stopped app and posts the job at once, before it listens | `studio/audio.rs` `wait_up`: poll `/health` for up to 120 s first |
| Ogg q5 suspected | not a cause: wav and ogg measure the same (peak -18.8 vs -18.8, same gaps) | none |

## A/B results

SFX "wood chop", 4 s (seed bug still in place, so both seeds of a row were the same clip):

| Setting | Time | RMS | Gaps | Spectral flatness | Verdict |
|---|---|---|---|---|---|
| raw (before) | 25 s | -44.0 | 12 | | quiet |
| 100 steps, cfg 4, normalised | 24 s | -28.2 | 0 | 0.06 | good |
| latent 5 s instead of 30 s | 8 s | -7.8 | 0 | 0.34 | noise: the model needs its 30 s latent |
| latent 10 s | 11 s | -29.0 | 13 | 0.47 | noise |
| 50 steps | 15 s | -28.0 | 0 | | same as 100 |
| 150 steps | 33 s | -28.6 | 0 | | same as 100 |
| cfg 3 | 24 s | -24.7 | 0 | 0.04 | same |
| cfg 6 | 24 s | -30.6 | 13 | | more holes |

With the seed fix, 3 seeds each on "wood chop" and "a wooden door creaks open": cfg 3 and cfg 4 give near
identical numbers per seed (RMS within 3.5 dB); the seed changes the result far more than cfg. The gaps
left in "wood chop" (0, 3 or 11 per seed) are the pauses between chops. Defaults stay at 100 steps, cfg 4.

Music "calm lofi piano loop for a cozy game menu", 20 s, one seed per row (two for base):

| Setting | Time | Length | Raw peak / flat | Normalised RMS | Verdict |
|---|---|---|---|---|---|
| cfg 1.5, temp 1.0, topk 50 (default) | 48 s | 20.1 s | 0.0 dB / 15.5 | -16.9 | good after the peak fix |
| cfg 3 | 46 s | 15.9 s | | -17.3 | ends early |
| temperature 0.8 | 38 s | 4.2 s | | -16.5 | ends after 4 s |
| topk 30 | 49 s | 20.1 s | | -19.7 | one gap |
| topk 100 | 38 s | 3.8 s | | -21.5 | ends after 4 s, 2 gaps |

HeartMuLa defaults stay. Not tried: the codec on the CPU (the clipping was in the save, not the codec) and
fp32 MOSS (bf16 needs 11.3 GiB already; fp32 does not fit 16 GB).

## Model limits

MOSS always denoises a 30 s latent and crops, so a 2 s clip costs as much as a 30 s one (about 24 s warm on the
9070 XT); shorter latents give noise. HeartMuLa sometimes ends a piece early at other sampler settings.
