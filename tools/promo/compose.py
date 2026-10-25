"""STU-P3: promotional banners a designer can edit.

A banner is built from a background, an optional subject (transparent PNG), the logo and two text lines, and is
written as OpenRaster (.ora: every part its own layer, opens in Krita and GIMP), SVG with live text (Inkscape,
Penpot) and a flat PNG. Text takes white or night, whichever reads best, and gets a dark scrim behind it when
neither reaches WCAG AA (headline 3:1 as large text, tagline 4.5:1). The Coder's draft (3 attempts) defined none
of the asked functions; Claude wrote this file (lesson 420). Text shrinks to fit its half of the banner and its
contrast is checked against the light and dark ends of what is under it, not the mean (lesson 421).

Run: python3 tools/promo/compose.py spec.json --out DIR [--stem banner] [--font Inter.ttf]
"""

import argparse
import base64
import io
import json
import os
import sys
import zipfile
from xml.sax.saxutils import escape, quoteattr

from PIL import Image, ImageFont

SIZES = {
    "repo-banner": (1280, 640),
    "social": (1200, 630),
    "steam-header": (460, 215),
    "steam-main": (616, 353),
    "discord": (960, 540),
}
PALETTE = {
    "night": "#0c0917",
    "plum": "#1c1329",
    "plum2": "#2c1f3f",
    "violet": "#5c398e",
    "lilac": "#cca9ff",
    "orange": "#f3941f",
    "mist": "#f4eefc",
    "white": "#ffffff",
    "black": "#000000",
}
TEXT_COLOURS = ["#ffffff", "#0c0917"]
NEED = {"headline": 3.0, "tagline": 4.5}


def rgb(hex_colour):
    h = hex_colour.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


def luminance(c):
    r, g, b = rgb(c) if isinstance(c, str) else c[:3]

    def ch(v):
        v /= 255
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4

    return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)


def contrast(a, b):
    la, lb = luminance(a), luminance(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


def text_colour(region):
    """The text colour with the best worst-case contrast. region is one (r, g, b) or a list of them (the light
    and dark ends of what sits under the text): the ratio is the lowest over that list.
    """
    pixels = [region] if isinstance(region[0], (int, float)) else list(region)

    def worst(c):
        return min(contrast(c, p) for p in pixels)

    best = max(TEXT_COLOURS, key=worst)
    return best, worst(best)


def fit_cover(img, w, h):
    s = max(w / img.width, h / img.height)
    img = img.resize((round(img.width * s), round(img.height * s)), Image.LANCZOS)
    x, y = (img.width - w) // 2, (img.height - h) // 2
    return img.crop((x, y, x + w, y + h))


def place_subject(sub, w, h, side="right", height_frac=0.9):
    margin = round(0.06 * h)
    nh = round(h * height_frac)
    nw = round(sub.width * nh / sub.height)
    resized = sub.resize((nw, nh), Image.LANCZOS)
    x = w - margin - nw if side == "right" else margin
    return x, h - nh, resized


def _canvas(w, h):
    return Image.new("RGBA", (w, h), (0, 0, 0, 0))


def _composite(layers, w, h):
    out = _canvas(w, h)
    for lay in layers:
        out = Image.alpha_composite(out, lay["image"])
    return out


def _ends(img, box):
    """The 5th and 95th percentile colours by luminance under box: a mean hides a light patch behind dark text."""
    small = img.crop(box).convert("RGB").resize((64, 16), Image.BOX)
    px = [small.getpixel((x, y)) for y in range(16) for x in range(64)]
    px.sort(key=luminance)
    return [px[len(px) // 20], px[len(px) * 19 // 20]]


def _fit_size(text, size, width, font_path):
    """The largest size up to size whose text fits width (steps of 2 px, at least 10)."""
    while size > 10:
        font = (
            ImageFont.truetype(font_path, size)
            if font_path
            else ImageFont.load_default(size)
        )
        if font.getlength(text) <= width:
            break
        size -= 2
    return size


def _open_image(path, what):
    """Open an input picture as RGBA, with a clear error when the file is not there. A PIL image is taken as it is
    (the Studio's banner node passes ComfyUI pictures straight in)."""
    if isinstance(path, Image.Image):
        return path.convert("RGBA")
    if not os.path.isfile(path):
        raise FileNotFoundError(f"{what} image not found: {path}")
    return Image.open(path).convert("RGBA")


def compose(spec):
    size = spec.get("size", "repo-banner")
    if size not in SIZES:
        raise ValueError("unknown size " + str(size))
    w, h = SIZES[size]
    margin = round(0.06 * h)
    side = spec.get("side") or "right"

    bg = spec.get("background") or PALETTE["plum"]
    if isinstance(bg, str) and bg.startswith("#"):
        background = Image.new("RGBA", (w, h), rgb(bg) + (255,))
    else:
        background = fit_cover(_open_image(bg, "background"), w, h)
    layers = [{"name": "background", "image": background}]
    if spec.get("subject"):
        x, y, s = place_subject(_open_image(spec["subject"], "subject"), w, h, side)
        c = _canvas(w, h)
        c.alpha_composite(s, (x, y))
        layers.append({"name": "subject", "image": c})
    if spec.get("logo"):
        logo = _open_image(spec["logo"], "logo")
        lh = round(0.18 * h)
        logo = logo.resize((round(logo.width * lh / logo.height), lh), Image.LANCZOS)
        c = _canvas(w, h)
        c.alpha_composite(logo, (margin, margin))
        layers.append({"name": "logo", "image": c})

    tx = margin if side == "right" else w // 2 + margin
    tw = w // 2 - 2 * margin
    hs = _fit_size(spec.get("headline", ""), round(0.11 * h), tw, spec.get("font"))
    ts = _fit_size(spec.get("tagline", ""), round(0.05 * h), tw, spec.get("font"))
    hy = round(0.42 * h)
    ty = hy + hs + round(0.03 * h)
    box = (tx, hy, tx + tw, ty + ts)

    colour, ratio = text_colour(_ends(_composite(layers, w, h), box))
    scrim = False
    if ratio < NEED["tagline"]:
        c = _canvas(w, h)
        pad = margin // 2
        c.paste(
            rgb(PALETTE["night"]) + (153,),
            (box[0] - pad, box[1] - pad, box[2] + pad, box[3] + pad),
        )
        layers.insert(1, {"name": "scrim", "image": c})
        colour, ratio = text_colour(_ends(_composite(layers, w, h), box))
        scrim = True

    texts = [
        {
            "name": "headline",
            "text": spec.get("headline", ""),
            "x": tx,
            "y": hy,
            "size": hs,
            "weight": 700,
            "colour": colour,
        },
        {
            "name": "tagline",
            "text": spec.get("tagline", ""),
            "x": tx,
            "y": ty,
            "size": ts,
            "weight": 400,
            "colour": colour,
        },
    ]
    r = round(ratio, 2)
    check = {
        name: {"colour": colour, "ratio": r, "need": need, "scrim": scrim}
        for name, need in NEED.items()
    }
    check["ok"] = all(ratio >= need for need in NEED.values())
    return {"width": w, "height": h, "layers": layers, "texts": texts, "check": check}


def render_text(b, name, font_path=None):
    t = next(t for t in b["texts"] if t["name"] == name)
    font = (
        ImageFont.truetype(font_path, t["size"])
        if font_path
        else ImageFont.load_default(t["size"])
    )
    c = _canvas(b["width"], b["height"])
    from PIL import ImageDraw

    ImageDraw.Draw(c).text(
        (t["x"], t["y"]), t["text"], font=font, fill=rgb(t["colour"]) + (255,)
    )
    return c


def flatten(b, font_path=None):
    out = _composite(b["layers"], b["width"], b["height"])
    for t in b["texts"]:
        out = Image.alpha_composite(out, render_text(b, t["name"], font_path))
    return out


def _png(img):
    buf = io.BytesIO()
    img.save(buf, "PNG")
    return buf.getvalue()


def write_ora(b, path, font_path=None):
    order = [t["name"] for t in reversed(b["texts"])] + [
        lay["name"] for lay in reversed(b["layers"])
    ]
    images = {lay["name"]: lay["image"] for lay in b["layers"]}
    for t in b["texts"]:
        images[t["name"]] = render_text(b, t["name"], font_path)
    stack = "".join(
        f'<layer name={quoteattr(n)} src="data/{n}.png" x="0" y="0" opacity="1.00" visibility="visible"/>'
        for n in order
    )
    xml = f'<?xml version="1.0" encoding="UTF-8"?><image version="0.0.3" w="{b["width"]}" h="{b["height"]}"><stack>{stack}</stack></image>'
    flat = flatten(b, font_path)
    thumb = flat.copy()
    thumb.thumbnail((256, 256))
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr(
            zipfile.ZipInfo("mimetype"),
            b"image/openraster",
            compress_type=zipfile.ZIP_STORED,
        )
        z.writestr("stack.xml", xml)
        for n in order:
            z.writestr(f"data/{n}.png", _png(images[n]))
        z.writestr("mergedimage.png", _png(flat))
        z.writestr("Thumbnails/thumbnail.png", _png(thumb))


def write_svg(b, path):
    w, h = b["width"], b["height"]
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">'
    ]
    for lay in b["layers"]:
        data = base64.b64encode(_png(lay["image"])).decode()
        parts.append(
            f'<image id="{lay["name"]}" x="0" y="0" width="{w}" height="{h}" href="data:image/png;base64,{data}"/>'
        )
    for t in b["texts"]:
        parts.append(
            f'<text id="{t["name"]}" x="{t["x"]}" y="{round(t["y"] + t["size"] * 0.8)}" font-family="Inter, sans-serif" '
            f'font-size="{t["size"]}" font-weight="{t["weight"]}" fill="{t["colour"]}">{escape(t["text"])}</text>'
        )
    parts.append("</svg>")
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(parts) + "\n")


def write_all(b, out_dir, stem, font_path=None):
    os.makedirs(out_dir, exist_ok=True)
    paths = [
        os.path.join(out_dir, f"{stem}.{ext}") for ext in ("ora", "svg", "png")
    ] + [os.path.join(out_dir, "check.json")]
    write_ora(b, paths[0], font_path)
    write_svg(b, paths[1])
    flatten(b, font_path).save(paths[2])
    with open(paths[3], "w", encoding="utf-8") as f:
        json.dump(b["check"], f, indent=2)
    return paths


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("spec")
    ap.add_argument("--out", required=True)
    ap.add_argument("--stem", default="banner")
    ap.add_argument("--font")
    a = ap.parse_args()
    with open(a.spec, encoding="utf-8") as f:
        b = compose({**json.load(f), "font": a.font})
    write_all(b, a.out, a.stem, a.font)
    print(json.dumps(b["check"]))
    sys.exit(0 if b["check"]["ok"] else 2)


if __name__ == "__main__":
    main()
