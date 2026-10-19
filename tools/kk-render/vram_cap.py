# kk-render: hard torch allocator cap for the Kompanion render container on soucouyant (16 GB RX 9070 XT, Kees's
# desktop). 0.50 = ~8 GB for torch; the host watchdog in kk_render.py kills the container if the card total passes 12 GB.
import torch
if torch.cuda.is_available():
    torch.cuda.set_per_process_memory_fraction(0.50, 0)
    print("kk-render: per-process VRAM fraction 0.50")
NODE_CLASS_MAPPINGS = {}
