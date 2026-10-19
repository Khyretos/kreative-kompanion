"""STU-SHEET3D: the fixed cell layouts of the Studio sheets. Cells are (name, x0, y0, x1, y1) in fractions of the sheet; ratio = width / height."""

from PIL import Image, ImageDraw

LAYOUTS = {
    "oc-v3": {
        "ratio": 2.5,
        "cells": [
            ("front", 0.0, 0.0, 0.2, 1.0),
            ("right", 0.2, 0.0, 0.4, 1.0),
            ("left", 0.4, 0.0, 0.6, 1.0),
            ("back", 0.6, 0.0, 0.8, 1.0),
            ("head-front", 0.8, 0.0, 0.9, 0.25),
            ("helmet-front", 0.9, 0.0, 1.0, 0.25),
            ("head-right", 0.8, 0.25, 0.9, 0.5),
            ("helmet-right", 0.9, 0.25, 1.0, 0.5),
            ("head-left", 0.8, 0.5, 0.9, 0.75),
            ("helmet-left", 0.9, 0.5, 1.0, 0.75),
            ("head-back", 0.8, 0.75, 0.9, 1.0),
            ("helmet-back", 0.9, 0.75, 1.0, 1.0),
        ],
    },
    "weapon": {
        "ratio": 2.0,
        "cells": [
            ("front", 0.0, 0.0, 0.25, 1.0),
            ("right", 0.25, 0.0, 0.5, 1.0),
            ("back", 0.5, 0.0, 0.75, 1.0),
            ("left", 0.75, 0.0, 1.0, 1.0),
        ],
    },
}


def line_width(height):
    return max(3, round(height * 0.006))


def cells_px(kind, width, height):
    layout = LAYOUTS[kind]
    result = []
    for name, x0, y0, x1, y1 in layout["cells"]:
        result.append(
            {"name": name, "box": (round(x0 * width), round(y0 * height), round(x1 * width), round(y1 * height))}
        )
    return result


def draw_lines(kind, width, height, fg=255, bg=0):
    img = Image.new("L", (width, height), bg)
    draw = ImageDraw.Draw(img)
    for cell in cells_px(kind, width, height):
        x0, y0, x1, y1 = cell["box"]
        draw.rectangle((x0, y0, x1 - 1, y1 - 1), outline=fg, width=line_width(height))
    return img


def region_mask(kind, width, height, prefix):
    img = Image.new("L", (width, height), 0)
    draw = ImageDraw.Draw(img)
    for cell in cells_px(kind, width, height):
        if cell["name"].startswith(prefix):
            x0, y0, x1, y1 = cell["box"]
            draw.rectangle((x0, y0, x1 - 1, y1 - 1), fill=255)
    return img
