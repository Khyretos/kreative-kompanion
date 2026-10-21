# kk-render: hard torch allocator cap for the Kompanion render container on soucouyant (16 GB RX 9070 XT, Kees's
# desktop). KK_VRAM_FRACTION per profile (sdxl 0.50, trellis 0.60); the host watchdog in kk_render.py kills the
# container if the card total passes 12 GB.
import os

import torch

if torch.cuda.is_available():
    f = float(os.environ.get("KK_VRAM_FRACTION", "0.50"))
    torch.cuda.set_per_process_memory_fraction(f, 0)
    print("kk-render: per-process VRAM fraction %.2f" % f)
NODE_CLASS_MAPPINGS = {}
