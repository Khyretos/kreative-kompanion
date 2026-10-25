import sys
from PIL import Image
import layout
W, H = 3200, 1280
for n in sys.argv[1:]:
    sheet = Image.open(f"sheet-{n}.png")
    for cell in layout.cells_px("oc-v3", W, H):
        if cell["name"].startswith("head-"):
            x0, y0, x1, y1 = cell["box"]; lw = layout.line_width(H)
            sheet.crop((x0 + lw, y0 + lw, x1 - lw, y1 - lw)).resize((512, 512), Image.LANCZOS).save(f"head-{n}-{cell['name'][5:]}.png")
