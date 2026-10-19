"""STU-SHEET3D: the OC sheet v3 and weapon sheet templates, built from the v2 sheet (no Blender).

Usage: sheet_v3.py oc V2_DIR OUT_DIR      (V2_DIR = studio/workflows/oc-sheet with the 1536x768 depth.png)
       sheet_v3.py weapon OUT_DIR
oc writes depth.png, pose.png, lines.png, helmet.png, regions.png (1600x640) and lines-final.png (3600x1440);
weapon writes lines.png (1344x672) and lines-final.png (3024x1512). Lines are white on black.
"""

import importlib.util
import os
import sys
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))


def _load(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, name + ".py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


layout = _load("layout")
v2 = _load("v2_points")
draw_pose = _load("draw_pose")

V2_SIZE = (1536, 768)
V2_INDEX = {"front": 0, "left": 1, "back": 2, "right": 3}
HEAD_Z = 1.62
HEAD_SIDE = 0.44
OC_SIZE = (1600, 640)
WEAPON_SIZE = (1344, 672)
# Regions for per-view prompts: (name, cells of layout "oc-v3", colour as 0xRRGGBB). A head row covers both its cells.
REGIONS = [
    ("front", ("front",), 0x101010),
    ("right", ("right",), 0x202020),
    ("left", ("left",), 0x303030),
    ("back", ("back",), 0x404040),
    ("head-front", ("head-front", "helmet-front"), 0x505050),
    ("head-right", ("head-right", "helmet-right"), 0x606060),
    ("head-left", ("head-left", "helmet-left"), 0x707070),
    ("head-back", ("head-back", "helmet-back"), 0x808080),
]


def regions_image(width=OC_SIZE[0], height=OC_SIZE[1]):
    img = Image.new("RGB", (width, height), (0, 0, 0))
    boxes = {c["name"]: c["box"] for c in layout.cells_px("oc-v3", width, height)}
    for _, cells, colour in REGIONS:
        rgb = ((colour >> 16) & 255, (colour >> 8) & 255, colour & 255)
        for name in cells:
            img.paste(rgb, boxes[name])
    return img


FINAL = 2.25


def v2_cell(view):
    i = V2_INDEX[view]
    w, h = V2_SIZE
    return (i * w // 4, 0, (i + 1) * w // 4, h)


def head_box_v2(i):
    w, h = V2_SIZE
    s = w / (v2.CELL * len(v2.VIEWS))
    cx = w / 2 + (i - (len(v2.VIEWS) - 1) / 2) * v2.CELL * s
    cy = h / 2 - (HEAD_Z - v2.CAM_Z) * s
    half = HEAD_SIDE * s / 2
    return (round(cx - half), round(cy - half), round(cx + half), round(cy + half))


def fit(src_box, dst_box):
    sw, sh = src_box[2] - src_box[0], src_box[3] - src_box[1]
    dw, dh = dst_box[2] - dst_box[0], dst_box[3] - dst_box[1]
    k = min(dw / sw, dh / sh)
    ox = dst_box[0] + (dw - sw * k) / 2
    oy = dst_box[1] + (dh - sh * k) / 2
    return k, ox, oy


def map_point(p, src_box, dst_box):
    if p is None:
        return None
    x, y = p
    x0, y0, x1, y1 = src_box
    if x < x0 or x >= x1 or y < y0 or y >= y1:
        return None
    k, ox, oy = fit(src_box, dst_box)
    return [round(ox + (p[0] - src_box[0]) * k, 1), round(oy + (p[1] - src_box[1]) * k, 1)]


def paste_cell(dst, src, src_box, dst_box):
    k, ox, oy = fit(src_box, dst_box)
    crop_w = max(1, round((src_box[2] - src_box[0]) * k))
    crop_h = max(1, round((src_box[3] - src_box[1]) * k))
    crop = src.crop(src_box).resize((crop_w, crop_h), Image.LANCZOS)
    dst.paste(crop, (round(ox), round(oy)))


def source_box(name):
    if name in ("front", "right", "left", "back"):
        return v2_cell(name)
    view = name.split("-")[1]
    return head_box_v2(V2_INDEX[view])


def build_oc(depth_v2, width=OC_SIZE[0], height=OC_SIZE[1]):
    depth_v2 = depth_v2.convert("RGB")
    pts = v2.points(*V2_SIZE)
    depth = Image.new("RGB", (width, height), (0, 0, 0))
    figures = []
    for cell in layout.cells_px("oc-v3", width, height):
        src = source_box(cell["name"])
        paste_cell(depth, depth_v2, src, cell["box"])
        view = cell["name"].split("-")[-1]
        figure = pts["figures"][V2_INDEX[view]]
        mapped_points = [map_point(p, src, cell["box"]) for p in figure["points"]]
        figures.append({"view": cell["name"], "points": mapped_points})
    points = {"width": width, "height": height, "order": pts["order"], "figures": figures}
    return {
        "depth": depth,
        "pose": draw_pose.draw(points),
        "lines": layout.draw_lines("oc-v3", width, height),
        "helmet": layout.region_mask("oc-v3", width, height, "helmet-"),
        "regions": regions_image(width, height),
        "points": points,
    }


def build_weapon(width=WEAPON_SIZE[0], height=WEAPON_SIZE[1]):
    depth = Image.new("RGB", (width, height), (0, 0, 0))
    for cell in layout.cells_px("weapon", width, height):
        if cell["name"] not in ("right", "left"):
            continue  # front and back: no depth guidance
        x0, y0, x1, y1 = cell["box"]
        cx, half = (x0 + x1) // 2, round((x1 - x0) * 0.04)
        top, bottom = y0 + round((y1 - y0) * 0.075), y1 - round((y1 - y0) * 0.075)
        depth.paste((180, 180, 180), (cx - half, top, cx + half + 1, bottom))
    return {"lines": layout.draw_lines("weapon", width, height), "depth": depth}


def main(argv):
    if len(argv) == 4 and argv[1] == "oc":
        parts, size, kind, out = build_oc(Image.open(os.path.join(argv[2], "depth.png"))), OC_SIZE, "oc-v3", argv[3]
    elif len(argv) == 3 and argv[1] == "weapon":
        parts, size, kind, out = build_weapon(), WEAPON_SIZE, "weapon", argv[2]
    else:
        print(__doc__.split("\n\n")[1].split("\noc writes")[0], file=sys.stderr)
        return 2
    os.makedirs(out, exist_ok=True)
    for key, img in parts.items():
        if key != "points":
            img.save(os.path.join(out, f"{key}.png"))
    final = (round(size[0] * FINAL), round(size[1] * FINAL))
    layout.draw_lines(kind, *final).save(os.path.join(out, "lines-final.png"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
