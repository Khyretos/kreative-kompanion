"""Tests for tools/pose-sheet/layout.py and v2_points.py (STU-SHEET3D). Run: python3 -m unittest tools/test_sheet_layout.py"""

import importlib.util
import os
import unittest

from PIL import Image, ImageChops

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name, rel):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, rel))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


lay = load("lay", "pose-sheet/layout.py")


class Layout(unittest.TestCase):
    def test_tables(self):
        oc = lay.LAYOUTS["oc-v3"]
        self.assertEqual(oc["ratio"], 2.5)
        names = [c[0] for c in oc["cells"]]
        self.assertEqual(names[:4], ["front", "right", "left", "back"])
        self.assertEqual(len(names), 12)
        self.assertEqual(names[4:6], ["head-front", "helmet-front"])
        self.assertEqual(names[-1], "helmet-back")
        self.assertEqual([c[0] for c in lay.LAYOUTS["weapon"]["cells"]], ["front", "right", "back", "left"])
        self.assertEqual(lay.LAYOUTS["weapon"]["ratio"], 2.0)

    def test_cells_cover_the_sheet(self):
        for kind in lay.LAYOUTS:
            area = sum((c[3] - c[1]) * (c[4] - c[2]) for c in lay.LAYOUTS[kind]["cells"])
            self.assertAlmostEqual(area, 1.0, places=6, msg=kind)

    def test_line_width(self):
        self.assertEqual(lay.line_width(640), 4)
        self.assertEqual(lay.line_width(100), 3)
        self.assertEqual(lay.line_width(1440), 9)

    def test_cells_px(self):
        cells = lay.cells_px("oc-v3", 1600, 640)
        self.assertEqual(cells[0], {"name": "front", "box": (0, 0, 320, 640)})
        self.assertEqual(cells[4], {"name": "head-front", "box": (1280, 0, 1440, 160)})
        self.assertEqual(cells[11], {"name": "helmet-back", "box": (1440, 480, 1600, 640)})
        self.assertEqual(lay.cells_px("weapon", 1344, 672)[2]["box"], (672, 0, 1008, 672))

    def test_draw_lines(self):
        img = lay.draw_lines("oc-v3", 1600, 640)
        self.assertEqual((img.mode, img.size), ("L", (1600, 640)))
        self.assertEqual(img.getpixel((0, 300)), 255)  # outer frame
        self.assertEqual(img.getpixel((3, 300)), 255)
        self.assertEqual(img.getpixel((4, 300)), 0)
        self.assertEqual(img.getpixel((160, 300)), 0)  # inside the front cell
        self.assertEqual(img.getpixel((319, 300)), 255)  # line between front and right
        self.assertEqual(img.getpixel((320, 300)), 255)
        self.assertEqual(img.getpixel((1360, 158)), 255)  # under head-front
        inv = lay.draw_lines("weapon", 1344, 672, fg=0, bg=255)
        self.assertEqual(inv.getpixel((0, 0)), 0)
        self.assertEqual(inv.getpixel((100, 300)), 255)

    def test_region_mask(self):
        m = lay.region_mask("oc-v3", 1600, 640, "helmet-")
        self.assertEqual((m.mode, m.size), ("L", (1600, 640)))
        self.assertEqual(m.getpixel((1500, 300)), 255)
        self.assertEqual(m.getpixel((1439, 300)), 0)
        self.assertEqual(m.getpixel((1440, 0)), 255)
        self.assertEqual(m.getpixel((100, 100)), 0)
        self.assertEqual(lay.region_mask("weapon", 100, 50, "nothing").getbbox(), None)

    def test_unknown_kind(self):
        with self.assertRaises(KeyError):
            lay.cells_px("nope", 10, 10)


class V2Points(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.vp = load("vp", "pose-sheet/v2_points.py")
        cls.dp = load("dp", "pose-sheet/draw_pose.py")

    def test_shape(self):
        data = self.vp.points()
        self.assertEqual((data["width"], data["height"]), (1536, 768))
        self.assertEqual([f["view"] for f in data["figures"]], ["front", "left", "back", "right"])
        for f in data["figures"]:
            self.assertEqual(len(f["points"]), 18)

    def test_front_neck_and_hidden_face(self):
        data = self.vp.points()
        front, back = data["figures"][0]["points"], data["figures"][2]["points"]
        # neck of the front figure: x = -1.5 * 1.15 m, z = 1.49 m; 1536 px over 4.6 m
        s = 1536 / 4.6
        self.assertAlmostEqual(front[1][0], 768 - 1.725 * s, delta=0.11)
        self.assertAlmostEqual(front[1][1], 384 - (1.49 - 0.92) * s, delta=0.11)
        self.assertIsNone(back[0])  # nose turned away
        self.assertIsNotNone(front[0])

    def test_matches_the_v2_pose_sheet(self):
        drawn = self.dp.draw(self.vp.points()).convert("L")
        ref = Image.open(os.path.join(HERE, "..", "studio", "workflows", "oc-sheet", "pose.png")).convert("L")
        diff = ImageChops.difference(drawn, ref).point(lambda v: 255 if v > 40 else 0)
        changed = sum(1 for v in diff.getdata() if v)
        self.assertLess(changed / (1536 * 768), 0.01)


if __name__ == "__main__":
    unittest.main()
