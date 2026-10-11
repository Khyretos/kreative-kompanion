"""Tests for tools/pose-sheet/draw_pose.py (STU-C1). Run: python3 -m unittest tools/test_draw_pose.py"""

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
PATH = os.path.join(HERE, "pose-sheet", "draw_pose.py")
spec = importlib.util.spec_from_file_location("dp", PATH)
dp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dp)

# One figure: nose, neck, right shoulder; everything else hidden.
PTS = [[100, 40], [100, 80], [60, 80]] + [None] * 15
DATA = {"width": 200, "height": 160, "figures": [{"view": "front", "points": PTS}]}


class DrawPose(unittest.TestCase):
    def test_tables(self):
        self.assertEqual(len(dp.COLORS), 18)
        self.assertEqual(dp.COLORS[0], (255, 0, 0))
        self.assertEqual(dp.COLORS[17], (255, 0, 85))
        self.assertEqual(len(dp.LIMBS), 17)
        self.assertEqual(dp.LIMBS[0], (1, 2))
        self.assertEqual(dp.LIMBS[12], (1, 0))

    def test_size_and_background(self):
        img = dp.draw(DATA)
        self.assertEqual(img.size, (200, 160))
        self.assertEqual(img.mode, "RGB")
        self.assertEqual(img.getpixel((5, 5)), (0, 0, 0))
        self.assertEqual(img.getpixel((195, 155)), (0, 0, 0))

    def test_joints_and_limbs(self):
        img = dp.draw(DATA)
        # Joint circles in full colour: neck (1) is (255, 85, 0), right shoulder (2) is (255, 170, 0).
        self.assertEqual(img.getpixel((100, 80)), (255, 85, 0))
        self.assertEqual(img.getpixel((60, 80)), (255, 170, 0))
        # Limb neck -> right shoulder (limb 0) at 60 % of its colour (255, 0, 0) halfway.
        self.assertEqual(img.getpixel((80, 80)), (153, 0, 0))
        # Limb nose -> neck (limb 12, colour (0, 0, 255)) halfway.
        self.assertEqual(img.getpixel((100, 60)), (0, 0, 153))
        # Hidden points draw nothing: no limb towards the left shoulder area.
        self.assertEqual(img.getpixel((140, 80)), (0, 0, 0))

    def test_stick_width_scales(self):
        self.assertEqual(dp.stick_width(512), 4)
        self.assertEqual(dp.stick_width(768), 6)
        self.assertEqual(dp.stick_width(256), 4)

    def test_cli(self):
        with tempfile.TemporaryDirectory() as d:
            src, out = os.path.join(d, "k.json"), os.path.join(d, "pose.png")
            with open(src, "w") as f:
                json.dump(DATA, f)
            r = subprocess.run(
                [sys.executable, PATH, src, out], capture_output=True, text=True
            )
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual(Image.open(out).size, (200, 160))


if __name__ == "__main__":
    unittest.main()
