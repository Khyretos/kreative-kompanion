"""Tests for tools/promo/compose.py (STU-P3: promotional banners a designer can edit: background, subject, logo and
text as separate layers, written as OpenRaster (.ora), SVG with live text and a flat PNG, text contrast WCAG AA).
Written before the code. Run: python3 tools/test_promo_compose.py"""

import importlib.util
import json
import os
import tempfile
import unittest
import zipfile
from xml.etree import ElementTree

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "pc", os.path.join(HERE, "promo", "compose.py")
)
pc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pc)


def tmp_png(d, name, size, colour, mode="RGB"):
    p = os.path.join(d, name)
    Image.new(mode, size, colour).save(p)
    return p


class Colour(unittest.TestCase):
    def test_palette_and_sizes(self):
        self.assertEqual(pc.PALETTE["night"], "#0c0917")
        self.assertEqual(pc.PALETTE["orange"], "#f3941f")
        self.assertEqual(pc.SIZES["repo-banner"], (1280, 640))
        self.assertEqual(pc.SIZES["social"], (1200, 630))
        self.assertEqual(pc.SIZES["steam-header"], (460, 215))

    def test_wcag_contrast(self):
        self.assertAlmostEqual(pc.contrast("#ffffff", "#000000"), 21.0, places=2)
        self.assertAlmostEqual(pc.contrast("#000000", "#ffffff"), 21.0, places=2)
        self.assertAlmostEqual(pc.contrast("#777777", "#777777"), 1.0, places=2)
        # palette.md: white on plum is above 15:1
        self.assertGreater(pc.contrast("#ffffff", "#1c1329"), 15)

    def test_text_colour_picks_the_readable_one(self):
        self.assertEqual(pc.text_colour((12, 9, 23))[0], "#ffffff")
        self.assertEqual(pc.text_colour((244, 238, 252))[0], "#0c0917")


class Geometry(unittest.TestCase):
    def test_fit_cover_fills_and_crops_the_middle(self):
        img = Image.new("RGB", (400, 100), (255, 0, 0))
        img.paste((0, 0, 255), (0, 0, 100, 100))  # blue left quarter is cropped away
        out = pc.fit_cover(img, 200, 200)
        self.assertEqual(out.size, (200, 200))
        self.assertEqual(out.getpixel((0, 100))[:3], (255, 0, 0))

    def test_subject_sits_on_the_bottom_of_its_side(self):
        sub = Image.new("RGBA", (100, 200), (0, 255, 0, 255))
        x, y, s = pc.place_subject(sub, 1280, 640, side="right", height_frac=0.9)
        self.assertEqual(s.size[1], 576)
        self.assertEqual(s.size[0], 288)
        self.assertEqual(y + s.size[1], 640)
        self.assertEqual(x + s.size[0], 1280 - round(0.06 * 640))
        x2, _, _ = pc.place_subject(sub, 1280, 640, side="left")
        self.assertEqual(x2, round(0.06 * 640))


class Compose(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.bg = tmp_png(self.d, "bg.png", (1600, 900), (28, 19, 41))
        sub = Image.new("RGBA", (300, 600), (0, 0, 0, 0))
        sub.paste((200, 120, 40, 255), (50, 50, 250, 600))
        self.sub = os.path.join(self.d, "sub.png")
        sub.save(self.sub)
        self.logo = tmp_png(self.d, "logo.png", (128, 128), (243, 148, 31, 255), "RGBA")

    def spec(self, **kw):
        s = {
            "size": "repo-banner",
            "background": self.bg,
            "subject": self.sub,
            "logo": self.logo,
            "headline": "kk-engine",
            "tagline": "A game engine you can read",
            "side": "right",
        }
        s.update(kw)
        return s

    def test_layers_and_texts(self):
        b = pc.compose(self.spec())
        self.assertEqual((b["width"], b["height"]), (1280, 640))
        self.assertEqual(
            [lay["name"] for lay in b["layers"]], ["background", "subject", "logo"]
        )
        for lay in b["layers"]:
            self.assertEqual(lay["image"].mode, "RGBA")
            self.assertEqual(lay["image"].size, (1280, 640))
        self.assertEqual([t["name"] for t in b["texts"]], ["headline", "tagline"])
        h, t = b["texts"]
        self.assertEqual(h["text"], "kk-engine")
        self.assertEqual(h["size"], round(0.11 * 640))
        self.assertEqual(t["size"], round(0.05 * 640))
        self.assertLess(h["x"] + 10, 640, "text sits on the side away from the subject")
        self.assertEqual(h["colour"], "#ffffff")
        self.assertGreaterEqual(b["check"]["headline"]["ratio"], 3.0)
        self.assertGreaterEqual(b["check"]["tagline"]["ratio"], 4.5)
        self.assertFalse(b["check"]["tagline"]["scrim"])

    def test_a_busy_mid_grey_background_gets_a_scrim(self):
        bg = tmp_png(
            self.d, "grey.png", (1280, 640), (119, 119, 119)
        )  # neither white (4.49) nor night (4.37) reaches 4.5
        b = pc.compose(self.spec(background=bg))
        self.assertTrue(b["check"]["tagline"]["scrim"])
        self.assertIn("scrim", [lay["name"] for lay in b["layers"]])
        self.assertGreaterEqual(b["check"]["tagline"]["ratio"], 4.5)

    def test_colour_background_and_no_subject(self):
        b = pc.compose(self.spec(background="#f4eefc", subject=None, size="social"))
        self.assertEqual((b["width"], b["height"]), (1200, 630))
        self.assertEqual([lay["name"] for lay in b["layers"]], ["background", "logo"])
        self.assertEqual(b["texts"][0]["colour"], "#0c0917")

    def test_unknown_size(self):
        with self.assertRaises(ValueError):
            pc.compose(self.spec(size="billboard"))

    def test_files(self):
        b = pc.compose(self.spec())
        out = tempfile.mkdtemp()
        paths = pc.write_all(b, out, "banner")
        self.assertEqual(
            sorted(os.path.basename(p) for p in paths),
            ["banner.ora", "banner.png", "banner.svg", "check.json"],
        )
        # OpenRaster: mimetype first and stored, stack.xml lists the layers top first, text as its own layers
        with zipfile.ZipFile(os.path.join(out, "banner.ora")) as z:
            names = z.namelist()
            self.assertEqual(names[0], "mimetype")
            self.assertEqual(z.getinfo("mimetype").compress_type, zipfile.ZIP_STORED)
            self.assertEqual(z.read("mimetype"), b"image/openraster")
            self.assertIn("mergedimage.png", names)
            self.assertIn("Thumbnails/thumbnail.png", names)
            stack = ElementTree.fromstring(z.read("stack.xml"))
            self.assertEqual(stack.get("w"), "1280")
            layers = [lay.get("name") for lay in stack.iter("layer")]
            self.assertEqual(
                layers, ["tagline", "headline", "logo", "subject", "background"]
            )
            for lay in stack.iter("layer"):
                self.assertIn(lay.get("src"), names)
        # SVG: live text, one image per picture layer
        svg = ElementTree.parse(os.path.join(out, "banner.svg")).getroot()
        ns = "{http://www.w3.org/2000/svg}"
        self.assertEqual(svg.get("width"), "1280")
        texts = [t.text for t in svg.iter(ns + "text")]
        self.assertEqual(texts, ["kk-engine", "A game engine you can read"])
        self.assertEqual(len(list(svg.iter(ns + "image"))), 3)
        self.assertIn("Inter", svg.find(ns + "text").get("font-family"))
        flat = Image.open(os.path.join(out, "banner.png"))
        self.assertEqual(flat.size, (1280, 640))
        check = json.load(open(os.path.join(out, "check.json")))
        self.assertTrue(check["ok"])


if __name__ == "__main__":
    unittest.main()
