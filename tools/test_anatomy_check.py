"""Tests for tools/anatomy/check.py (STU-C4). Run: python3 -m unittest tools/test_anatomy_check.py"""

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import types
import unittest
from typing import ClassVar

HERE = os.path.dirname(os.path.abspath(__file__))
PATH = os.path.join(HERE, "anatomy", "check.py")
spec = importlib.util.spec_from_file_location("ac", PATH)
ac = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ac)

W, H = 400, 200  # four cells of 100 px: front, left, back, right


def box(cx, cy, size=20, conf=0.9):
    return [cx - size / 2, cy - size / 2, cx + size / 2, cy + size / 2, conf]


GOOD_FACES = [box(50, 30), box(150, 30), box(350, 30)]
GOOD_HANDS = [
    box(30, 100),
    box(70, 100),
    box(140, 100),
    box(240, 100),
    box(260, 100),
    box(360, 100),
]


class Judge(unittest.TestCase):
    def test_a_good_sheet_passes(self):
        r = ac.judge(GOOD_FACES, GOOD_HANDS, W, H)
        self.assertTrue(r["ok"])
        self.assertEqual(r["issues"], [])
        self.assertEqual(
            [c["view"] for c in r["cells"]], ["front", "left", "back", "right"]
        )
        self.assertEqual([c["faces"] for c in r["cells"]], [1, 1, 0, 1])
        self.assertEqual([c["hands"] for c in r["cells"]], [2, 1, 2, 1])

    def test_a_face_in_the_back_view_fails(self):
        r = ac.judge(GOOD_FACES + [box(250, 30)], GOOD_HANDS, W, H)
        self.assertFalse(r["ok"])
        self.assertIn("back: face seen (head facing the wrong way)", r["issues"])

    def test_no_face_in_the_front_view_fails(self):
        r = ac.judge(GOOD_FACES[1:], GOOD_HANDS, W, H)
        self.assertIn("front: no face (head turned away)", r["issues"])

    def test_two_faces_in_one_view_fail(self):
        r = ac.judge(GOOD_FACES + [box(150, 60)], GOOD_HANDS, W, H)
        self.assertIn("left: 2 faces", r["issues"])

    def test_three_hands_fail(self):
        r = ac.judge(GOOD_FACES, GOOD_HANDS + [box(50, 150)], W, H)
        self.assertIn("front: 3 hands", r["issues"])

    def test_two_hands_at_one_wrist_fail(self):
        r = ac.judge(GOOD_FACES, GOOD_HANDS + [box(144, 104)], W, H)
        self.assertIn("left: two hands at one wrist", r["issues"])

    def test_low_confidence_boxes_are_ignored(self):
        r = ac.judge(
            GOOD_FACES + [box(250, 30, conf=0.2)],
            GOOD_HANDS + [box(50, 150, conf=0.2)],
            W,
            H,
        )
        self.assertTrue(r["ok"])

    def test_the_score_counts_issues(self):
        r = ac.judge(GOOD_FACES[1:] + [box(250, 30)], GOOD_HANDS, W, H)
        self.assertEqual(r["score"], 2)
        self.assertEqual(ac.judge(GOOD_FACES, GOOD_HANDS, W, H)["score"], 0)

    def test_side_views_can_be_lenient(self):
        r = ac.judge([box(50, 30)], GOOD_HANDS, W, H, face_views=("front",))
        self.assertTrue(r["ok"])


class FakeList(list):
    def tolist(self):
        return list(self)


class FakeYOLO:
    loads: ClassVar[list] = []
    calls: ClassVar[list] = []

    def __init__(self, path):
        FakeYOLO.loads.append(os.path.basename(path))
        self.name = os.path.basename(path)

    def __call__(self, img, conf=0.25, imgsz=640, verbose=True):
        FakeYOLO.calls.append((self.name, img.size, conf, imgsz))
        boxes = types.SimpleNamespace(
            xyxy=FakeList([[10.0, 20.0, 30.0, 40.0]]), conf=FakeList([0.9])
        )
        return [types.SimpleNamespace(boxes=boxes)]


try:
    import PIL
except ImportError:
    PIL = None


@unittest.skipIf(PIL is None, "needs Pillow")
class Detect(unittest.TestCase):
    def test_each_quarter_is_checked_and_boxes_shift_back(self):
        from PIL import Image

        sys.modules["ultralytics"] = types.SimpleNamespace(YOLO=FakeYOLO)
        FakeYOLO.loads, FakeYOLO.calls = [], []
        with tempfile.TemporaryDirectory() as d:
            img = os.path.join(d, "sheet.png")
            Image.new("RGB", (400, 200)).save(img)
            faces, hands, w, h = ac.detect(img, d)
            ac.detect(img, d)
        self.assertEqual((w, h), (400, 200))
        self.assertEqual(sorted(FakeYOLO.loads), ["face_yolov8m.pt", "hand_yolov8s.pt"])
        self.assertEqual(len(FakeYOLO.calls), 16)
        self.assertEqual(FakeYOLO.calls[0][1], (100, 200))
        self.assertEqual([f[0] for f in faces], [10.0, 110.0, 210.0, 310.0])
        self.assertEqual(faces[1], [110.0, 20.0, 130.0, 40.0, 0.9])
        self.assertEqual(len(hands), 4)


class Cli(unittest.TestCase):
    def test_help_runs_without_ultralytics(self):
        p = subprocess.run(
            [sys.executable, PATH, "--help"],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("--serve", p.stdout)

    def test_judge_json_reads_boxes(self):
        data = json.dumps(
            {"width": W, "height": H, "faces": GOOD_FACES, "hands": GOOD_HANDS}
        )
        p = subprocess.run(
            [sys.executable, PATH, "--judge-json", "-"],
            input=data,
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertTrue(json.loads(p.stdout)["ok"])

    def test_a_missing_image_exits_1(self):
        p = subprocess.run(
            [sys.executable, PATH, "/nonexistent/a.png", "--models", "/nonexistent"],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        self.assertEqual(p.returncode, 1)
        self.assertFalse(json.loads(p.stdout.splitlines()[0])["ok"])


if __name__ == "__main__":
    unittest.main()
