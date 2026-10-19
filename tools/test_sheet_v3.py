"""Tests for tools/pose-sheet/sheet_v3.py (STU-SHEET3D). Run: python3 -m unittest tools/test_sheet_v3.py"""

import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "pose-sheet", "sheet_v3.py")
V2 = os.path.join(HERE, "..", "studio", "workflows", "oc-sheet")
spec = importlib.util.spec_from_file_location("sv3", SCRIPT)
sv3 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sv3)


class Geometry(unittest.TestCase):
    def test_v2_order(self):
        self.assertEqual(sv3.V2_INDEX, {"front": 0, "left": 1, "back": 2, "right": 3})

    def test_head_box(self):
        s = 1536 / 4.6
        cx, cy, half = 768 - 1.5 * 1.15 * s, 384 - (1.62 - 0.92) * s, 0.22 * s
        box = sv3.head_box_v2(0)
        for got, want in zip(box, (cx - half, cy - half, cx + half, cy + half)):
            self.assertAlmostEqual(got, want, delta=1)
        self.assertAlmostEqual(sv3.head_box_v2(3)[0] - box[0], 3 * 1.15 * s, delta=1)

    def test_map_point(self):
        self.assertEqual(sv3.map_point([192, 384], (0, 0, 384, 768), (0, 0, 320, 640)), [160.0, 320.0])
        self.assertEqual(sv3.map_point([10, 10], (0, 0, 100, 100), (200, 0, 300, 200)), [210.0, 60.0])  # centred
        self.assertIsNone(sv3.map_point(None, (0, 0, 10, 10), (0, 0, 10, 10)))
        self.assertIsNone(sv3.map_point([50, 5], (0, 0, 10, 10), (0, 0, 10, 10)))

    def test_paste_cell(self):
        src = Image.new("RGB", (100, 100), (0, 0, 0))
        src.paste((200, 100, 50), (0, 0, 50, 100))
        dst = Image.new("RGB", (400, 200), (0, 0, 0))
        sv3.paste_cell(dst, src, (0, 0, 100, 100), (200, 0, 300, 200))
        self.assertEqual(dst.getpixel((210, 100)), (200, 100, 50))
        self.assertEqual(dst.getpixel((290, 100)), (0, 0, 0))
        self.assertEqual(dst.getpixel((250, 10)), (0, 0, 0))  # letterbox above the square crop


class Build(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.out = sv3.build_oc(Image.open(os.path.join(V2, "depth.png")), 1600, 640)

    def test_parts(self):
        self.assertEqual(set(self.out), {"depth", "pose", "lines", "helmet", "regions", "points"})
        for k in ("depth", "pose", "lines", "helmet", "regions"):
            self.assertEqual(self.out[k].size, (1600, 640), k)
        self.assertEqual((self.out["lines"].mode, self.out["helmet"].mode), ("L", "L"))

    def test_depth_cells(self):
        d = self.out["depth"].convert("L")
        self.assertGreater(d.getpixel((160, 320)), 100)  # front torso
        self.assertGreater(d.getpixel((1360, 80)), 100)  # head-front
        self.assertEqual(d.getpixel((1360, 80)), d.getpixel((1520, 80)))  # helmet cell = same head
        self.assertLess(d.getpixel((20, 320)), 40)  # background

    def test_lines_and_mask(self):
        self.assertEqual(self.out["lines"].getpixel((0, 0)), 255)
        self.assertEqual(self.out["lines"].getpixel((160, 600)), 0)
        self.assertEqual(self.out["helmet"].getpixel((1500, 300)), 255)
        self.assertEqual(self.out["helmet"].getpixel((1360, 300)), 0)

    def test_points(self):
        pts = self.out["points"]
        self.assertEqual((pts["width"], pts["height"]), (1600, 640))
        names = [f["view"] for f in pts["figures"]]
        self.assertEqual(names[:5], ["front", "right", "left", "back", "head-front"])
        self.assertEqual(len(names), 12)
        neck = pts["figures"][0]["points"][1]
        s = 1536 / 4.6
        self.assertAlmostEqual(neck[0], 160, delta=0.5)
        self.assertAlmostEqual(neck[1], (384 - 0.57 * s) * 640 / 768, delta=0.5)
        head = pts["figures"][4]["points"]
        self.assertIsNotNone(head[0])  # nose in the head cell
        self.assertIsNone(head[4])  # the wrist is outside the head crop
        self.assertGreater(max(self.out["pose"].convert("L").crop((150, 150, 170, 172)).getdata()), 0)

    def test_weapon(self):
        w = sv3.build_weapon(1344, 672)
        self.assertEqual(set(w), {"lines"})
        self.assertEqual(w["lines"].size, (1344, 672))



class Regions(unittest.TestCase):
    def test_table(self):
        self.assertEqual([r[0] for r in sv3.REGIONS], ["front", "right", "left", "back", "head-front", "head-right", "head-left", "head-back"])
        self.assertEqual(sv3.REGIONS[0], ("front", ("front",), 0x101010))
        self.assertEqual(sv3.REGIONS[5], ("head-right", ("head-right", "helmet-right"), 0x606060))

    def test_image(self):
        img = sv3.regions_image(1600, 640)
        self.assertEqual((img.mode, img.size), ("RGB", (1600, 640)))
        self.assertEqual(img.getpixel((160, 320)), (0x10, 0x10, 0x10))
        self.assertEqual(img.getpixel((480, 320)), (0x20, 0x20, 0x20))
        self.assertEqual(img.getpixel((1360, 240)), (0x60, 0x60, 0x60))
        self.assertEqual(img.getpixel((1520, 240)), (0x60, 0x60, 0x60))  # the helmet cell of the row too
        self.assertEqual(img.getpixel((1520, 600)), (0x80, 0x80, 0x80))
        self.assertEqual(len(set(img.getdata())), 8)  # every pixel belongs to one region

class Cli(unittest.TestCase):
    def test_writes_the_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = subprocess.run([sys.executable, SCRIPT, "oc", V2, tmp], capture_output=True, text=True, timeout=120)
            self.assertEqual(p.returncode, 0, p.stderr)
            for f in ("depth.png", "pose.png", "lines.png", "lines-final.png", "helmet.png", "regions.png"):
                self.assertTrue(os.path.exists(os.path.join(tmp, f)), f)
            self.assertEqual(Image.open(os.path.join(tmp, "lines-final.png")).size, (3600, 1440))
            wdir = os.path.join(tmp, "w")
            p = subprocess.run([sys.executable, SCRIPT, "weapon", wdir], capture_output=True, text=True, timeout=120)
            self.assertEqual(p.returncode, 0, p.stderr)
            self.assertEqual(Image.open(os.path.join(wdir, "lines.png")).size, (1344, 672))
            self.assertEqual(Image.open(os.path.join(wdir, "lines-final.png")).size, (3024, 1512))
            p = subprocess.run([sys.executable, SCRIPT], capture_output=True, text=True, timeout=60)
            self.assertEqual(p.returncode, 2)


if __name__ == "__main__":
    unittest.main()
