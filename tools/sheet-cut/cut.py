"""STU-SHEET3D: cut a Studio sheet into its cells by the fixed layout (tools/pose-sheet/layout.py).

Usage: cut.py SHEET.png KIND OUT_DIR   (KIND: oc-v3 or weapon)
Writes OUT_DIR/<cell>.png for every cell with content and OUT_DIR/cut.json; prints the cells whose figure
touches the cell border (it may be cut off there)."""

import importlib.util
import json
import os
import sys
from PIL import Image, ImageChops

_LAYOUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "pose-sheet", "layout.py")
_spec = importlib.util.spec_from_file_location("sheet_layout", _LAYOUT)
layout = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(layout)

MARGIN = 2
PAD = 8
TOL = 24


def inner_box(box, lw, margin=MARGIN):
    x0, y0, x1, y1 = box
    return (x0 + lw + margin, y0 + lw + margin, x1 - lw - margin, y1 - lw - margin)


def trim(crop, tol=TOL):
    bg = crop.getpixel((0, 0))
    diff = ImageChops.difference(crop, Image.new("RGB", crop.size, bg)).convert("L").point(lambda v: 255 if v > tol else 0)
    bbox = diff.getbbox()
    if bbox is None:
        return None
    touches = bbox[0] <= 0 or bbox[1] <= 0 or bbox[2] >= crop.width or bbox[3] >= crop.height
    return (bbox, touches)


def cut(img, kind, margin=MARGIN):
    img = img.convert("RGB")
    lw = layout.line_width(img.height)
    cells = layout.cells_px(kind, img.width, img.height)
    result = []
    for cell in cells:
        inner = inner_box(cell["box"], lw, margin)
        crop = img.crop(inner)
        t = trim(crop)
        if t is None:
            trimmed = inner
            touches = False
            empty = True
        else:
            bbox, touches = t
            trimmed = (
                max(inner[0], inner[0] + bbox[0] - PAD),
                max(inner[1], inner[1] + bbox[1] - PAD),
                min(inner[2], inner[0] + bbox[2] + PAD),
                min(inner[3], inner[1] + bbox[3] + PAD),
            )
            empty = False
        result.append(
            {
                "name": cell["name"],
                "box": cell["box"],
                "inner": inner,
                "trimmed_box": trimmed,
                "touches": touches,
                "empty": empty,
                "image": img.crop(trimmed),
            }
        )
    return result


def main(argv):
    if len(argv) != 4 or argv[2] not in layout.LAYOUTS:
        print(f"usage: cut.py SHEET.png KIND OUT_DIR (KIND: {', '.join(layout.LAYOUTS)})", file=sys.stderr)
        return 2
    img_path, kind, out_dir = argv[1], argv[2], argv[3]
    img = Image.open(img_path)
    cells = cut(img, kind)
    os.makedirs(out_dir, exist_ok=True)
    for c in cells:
        if not c["empty"]:
            path = os.path.join(out_dir, c["name"] + ".png")
            c["image"].save(path)
    data = {
        "sheet": os.path.basename(argv[1]),
        "kind": kind,
        "size": [img.width, img.height],
        "cells": [
            {"name": c["name"], "box": list(c["box"]), "inner": list(c["inner"]), "trimmed_box": list(c["trimmed_box"]), "touches": c["touches"], "empty": c["empty"]}
            for c in cells
        ],
    }
    with open(os.path.join(out_dir, "cut.json"), "w") as f:
        json.dump(data, f, indent=1)
    for c in cells:
        if c["touches"]:
            print(f"{c['name']}: figure touches the cell border")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
