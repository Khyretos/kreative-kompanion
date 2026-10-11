"""STU-C1's pose drawer for the four-view character sheet."""

import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw

COLORS = [
    (255, 0, 0),
    (255, 85, 0),
    (255, 170, 0),
    (255, 255, 0),
    (170, 255, 0),
    (85, 255, 0),
    (0, 255, 0),
    (0, 255, 85),
    (0, 255, 170),
    (0, 255, 255),
    (0, 170, 255),
    (0, 85, 255),
    (0, 0, 255),
    (85, 0, 255),
    (170, 0, 255),
    (255, 0, 255),
    (255, 0, 170),
    (255, 0, 85),
]

LIMBS = [
    (1, 2),
    (1, 5),
    (2, 3),
    (3, 4),
    (5, 6),
    (6, 7),
    (1, 8),
    (8, 9),
    (9, 10),
    (1, 11),
    (11, 12),
    (12, 13),
    (1, 0),
    (0, 14),
    (14, 16),
    (0, 15),
    (15, 17),
]


def stick_width(height: int) -> int:
    return max(4, round(4 * height / 512))


def draw(data: dict) -> Image.Image:
    width = data["width"]
    height = data["height"]
    figures = data.get("figures", [])

    img = Image.new("RGB", (width, height), (0, 0, 0))
    draw_layer = ImageDraw.Draw(img)

    w = stick_width(height)

    # Draw limbs first
    for fig in figures:
        points = fig.get("points", [])
        for i, (a, b) in enumerate(LIMBS):
            p_a = points[a]
            p_b = points[b]
            if p_a is None or p_b is None:
                continue
            x1, y1 = p_a
            x2, y2 = p_b
            cx = (x1 + x2) / 2
            cy = (y1 + y2) / 2
            dx = x2 - x1
            dy = y2 - y1
            dist = math.hypot(dx, dy)
            half_len = dist / 2
            angle = math.atan2(dy, dx)

            # Ellipse: 36 points approximated by drawing a rotated ellipse
            pts_ellipse = []
            for k in range(36):
                t = 2 * math.pi * k / 36
                rx = half_len
                ry = w
                ex = rx * math.cos(t)
                ey = ry * math.sin(t)
                px = cx + ex * math.cos(angle) - ey * math.sin(angle)
                py = cy + ex * math.sin(angle) + ey * math.cos(angle)
                pts_ellipse.append((px, py))

            color = tuple(int(c * 0.6) for c in COLORS[i])
            draw_layer.polygon(pts_ellipse, fill=color)

    # Draw joints after limbs
    for fig in figures:
        points = fig.get("points", [])
        for j, p in enumerate(points):
            if p is None:
                continue
            x, y = p
            draw_layer.ellipse(
                (x - w, y - w, x + w, y + w),
                fill=COLORS[j],
            )

    return img


def main():
    if len(sys.argv) != 3:
        print("Usage: python3 draw_pose.py <input.json> <output.png>", file=sys.stderr)
        sys.exit(2)

    input_path = Path(sys.argv[1])
    output_path = Path(sys.argv[2])

    try:
        with open(input_path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except Exception as e:
        print(f"Error reading JSON: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        img = draw(data)
        img.save(output_path, "PNG")
    except Exception as e:
        print(f"Error drawing image: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
