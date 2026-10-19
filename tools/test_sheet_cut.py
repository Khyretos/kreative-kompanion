"""Tests for tools/sheet-cut/cut.py (STU-SHEET3D). Run: python3 -m unittest tools/test_sheet_cut.py"""

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest

from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
CUT = os.path.join(HERE, "sheet-cut", "cut.py")
spec = importlib.util.spec_from_file_location("cut", CUT)
cut = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cut)


def sheet():
    """A 1600x640 oc-v3 sheet: white, black lines, a red figure inside 'front', a blue one touching the
    inner edge of 'back', a green head in 'head-front'; every other cell empty."""
    img = Image.new("RGB", (1600, 640), (255, 255, 255))
    lines = cut.layout.draw_lines("oc-v3", 1600, 640)
    img.paste((0, 0, 0), mask=lines)
    d = ImageDraw.Draw(img)
    d.rectangle((100, 100, 199, 499), fill=(200, 30, 30))
    d.rectangle((960, 50, 1100, 600), fill=(30, 30, 200))  # back cell is 960-1280: crosses its inner edge (966)
    d.rectangle((1330, 40, 1389, 119), fill=(30, 160, 30))
    return img


class Cut(unittest.TestCase):
    def test_inner_box(self):
        # line width 4 at 640 px, margin 2: 6 px inside each side of the cell
        self.assertEqual(cut.inner_box((0, 0, 320, 640), 4, 2), (6, 6, 314, 634))

    def test_cells(self):
        res = {c["name"]: c for c in cut.cut(sheet(), "oc-v3")}
        self.assertEqual(len(res), 12)
        front = res["front"]
        self.assertFalse(front["touches"])
        self.assertFalse(front["empty"])
        # red figure 100-199 x 100-499 plus 8 px padding, in sheet pixels
        self.assertEqual(front["trimmed_box"], (92, 92, 208, 508))
        self.assertEqual(front["image"].size, (116, 416))
        self.assertTrue(res["back"]["touches"])
        self.assertFalse(res["head-front"]["touches"])
        self.assertTrue(res["helmet-left"]["empty"])
        self.assertEqual(res["helmet-left"]["trimmed_box"], res["helmet-left"]["inner"])
        self.assertNotIn((0, 0, 0), set(front["image"].getdata()), "no separator line in the crop")

    def test_weapon_kind(self):
        img = Image.new("RGB", (1344, 672), (255, 255, 255))
        self.assertEqual([c["name"] for c in cut.cut(img, "weapon")], ["front", "right", "back", "left"])
        self.assertTrue(all(c["empty"] for c in cut.cut(img, "weapon")))

    def test_cli(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = os.path.join(tmp, "sheet.png")
            sheet().save(src)
            out = os.path.join(tmp, "out")
            p = subprocess.run([sys.executable, CUT, src, "oc-v3", out], capture_output=True, text=True, timeout=60)
            self.assertEqual(p.returncode, 0, p.stderr)
            self.assertIn("back", p.stdout)  # touching cells are named
            data = json.load(open(os.path.join(out, "cut.json")))
            self.assertEqual(data["kind"], "oc-v3")
            self.assertEqual([c["name"] for c in data["cells"]][:4], ["front", "right", "left", "back"])
            self.assertEqual(data["cells"][0]["trimmed_box"], [92, 92, 208, 508])
            self.assertTrue(os.path.exists(os.path.join(out, "front.png")))
            self.assertFalse(os.path.exists(os.path.join(out, "helmet-left.png")), "empty cells are not written")
            p = subprocess.run([sys.executable, CUT, src, "nope", out], capture_output=True, text=True, timeout=60)
            self.assertEqual(p.returncode, 2)


if __name__ == "__main__":
    unittest.main()
