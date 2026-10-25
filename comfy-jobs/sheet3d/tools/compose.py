# STU-SHEET3D v3 sheet from MV views refined by SDXL: body cells front/right/left/back, head grid from the same views.
import sys
import numpy as np
from PIL import Image, ImageDraw
import layout

ORDER = {"front": 1, "right": 2, "back": 3, "left": 4}  # MV-Adapter view order
W, H = 3200, 1280

def figure_top(im):
    a = np.asarray(im.convert("RGB")).astype(int)
    m = np.abs(a - a[4, 4]).sum(2) > 30
    ys, xs = np.where(m)
    return ys.min(), ys.max(), xs.min(), xs.max()

for n in sys.argv[1:]:
    sheet = Image.new("RGB", (W, H), (200, 200, 200))
    views = {v: Image.open(f"refined-{n}-{i}_00001_.png").convert("RGB") for v, i in ORDER.items()}
    for cell in layout.cells_px("oc-v3", W, H):
        name = cell["name"]; x0, y0, x1, y1 = cell["box"]; cw, chh = x1 - x0, y1 - y0
        view = name.split("-")[-1]
        im = views[view]
        if "-" not in name:
            src = im.resize((cw, chh), Image.LANCZOS)
        else:
            t, b, l, r = figure_top(im)
            side = int((b - t) * 0.24)
            cx = (l + r) // 2 if view in ("front", "back") else None
            # head x from the top rows of the figure
            a = np.asarray(im).astype(int); m = np.abs(a - a[4, 4]).sum(2) > 30
            rows = m[t:t + side]
            xs = np.where(rows.any(0))[0]
            cx = (xs.min() + xs.max()) // 2
            box = (cx - side // 2, max(0, t - side // 10), cx + side // 2, max(0, t - side // 10) + side)
            src = im.crop(box).resize((cw, chh), Image.LANCZOS)
        sheet.paste(src, (x0, y0))
    lines = layout.draw_lines("oc-v3", W, H)
    sheet.paste((0, 0, 0), mask=lines)
    sheet.save(f"sheet-{n}.png")
    sheet.resize((W // 4, H // 4), Image.LANCZOS).save(f"sheet-{n}-small.png")
    print("wrote", n)
