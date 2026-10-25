import sys
from PIL import Image
import layout
W, H = 3200, 1280
for n in sys.argv[1:]:
    sheet = Image.open(f"sheet-{n}.png")
    for cell in layout.cells_px("oc-v3", W, H):
        if cell["name"].startswith("helmet-"):
            x0, y0, x1, y1 = cell["box"]; lw = layout.line_width(H)
            v = cell["name"][7:]
            sheet.paste(Image.open(f"helmet-{n}-{v}_00001_.png").convert("RGB").resize((x1 - x0 - 2 * lw, y1 - y0 - 2 * lw), Image.LANCZOS), (x0 + lw, y0 + lw))
    sheet.save(f"sheet-{n}.png"); sheet.resize((W // 4, H // 4), Image.LANCZOS).save(f"sheet-{n}-small.png")
