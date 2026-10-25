# black cell lines of the oc-v3 layout on a transparent 3200x1280 PNG (the sheet graph composites it on top)
from PIL import Image
import layout
W, H = 3200, 1280
lines = layout.draw_lines("oc-v3", W, H)  # L: 255 on the lines
img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
img.putalpha(lines)
img.save("lines.png")
