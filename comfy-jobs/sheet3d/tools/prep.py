# crop each MV view to its figure (grey background) and pad to 1:2 for the 640x1280 refine
import sys
from PIL import Image
import numpy as np
for n in sys.argv[1:]:
    for i in range(1, 5):
        im = Image.open(f"{n}_0000{i}_.png").convert("RGB")
        a = np.asarray(im).astype(int)
        bg = a[2, 2]
        m = (np.abs(a - bg).sum(2) > 30)
        ys, xs = np.where(m)
        x0, x1, y0, y1 = xs.min(), xs.max(), ys.min(), ys.max()
        h = int((y1 - y0) * 1.08); w = h // 2
        cx, cy = (x0 + x1) // 2, (y0 + y1) // 2
        canvas = Image.new("RGB", (w, h), tuple(int(v) for v in bg))
        crop = im.crop((x0, y0, x1 + 1, y1 + 1))
        canvas.paste(crop, ((w - crop.width) // 2, (h - crop.height) // 2))
        canvas.resize((320, 640), Image.LANCZOS).save(f"view-{n.split('-')[-1]}-{i}.png")
        print(n, i, (x0, y0, x1, y1))
