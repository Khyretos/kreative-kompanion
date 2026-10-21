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


# STU-SHEET3D: the v3 sheet, 1000x400: body cells of 200 px (front, right, left, back), then the head grid
# (x 800-900 bare head, 900-1000 helmet; rows of 100 px: front, right, left, back).
W3, H3 = 1000, 400
V3_FACES = [
    box(100, 40),
    box(300, 40),
    box(500, 40),
    box(850, 50),
    box(850, 150),
    box(850, 250),
]
V3_HANDS = [
    box(60, 200),
    box(140, 200),
    box(300, 200),
    box(500, 200),
    box(660, 200),
    box(740, 200),
]


class JudgeV3(unittest.TestCase):
    def test_a_good_v3_sheet_passes(self):
        r = ac.judge(V3_FACES, V3_HANDS, W3, H3, cells=ac.CELLS["oc-v3"])
        self.assertEqual(r["issues"], [])
        self.assertTrue(r["ok"])
        self.assertEqual(
            [c["view"] for c in r["cells"]][:5],
            ["front", "right", "left", "back", "head-front"],
        )
        self.assertEqual(len(r["cells"]), 12)
        self.assertEqual([c["faces"] for c in r["cells"]][:8], [1, 1, 1, 0, 1, 0, 1, 0])

    def test_v2_cells_are_the_default(self):
        self.assertEqual(
            [c[0] for c in ac.CELLS["v2"]], ["front", "left", "back", "right"]
        )
        self.assertEqual(
            ac.judge(GOOD_FACES, GOOD_HANDS, W, H),
            ac.judge(GOOD_FACES, GOOD_HANDS, W, H, cells=ac.CELLS["v2"]),
        )

    def test_head_cells(self):
        r = ac.judge(
            V3_FACES[:3] + [box(850, 350), box(850, 150)],
            V3_HANDS,
            W3,
            H3,
            cells=ac.CELLS["oc-v3"],
        )
        self.assertIn("head-front: no face (head turned away)", r["issues"])
        self.assertIn("head-left: no face (head turned away)", r["issues"])
        self.assertIn("head-back: face seen (head facing the wrong way)", r["issues"])
        self.assertEqual(r["score"], 3)

    def test_helmet_cells_and_hands_in_head_cells_are_not_judged(self):
        extra_faces = [box(950, 50), box(950, 60), box(950, 350)]
        extra_hands = [box(850, 80), box(860, 80), box(870, 80)]
        r = ac.judge(
            V3_FACES + extra_faces,
            V3_HANDS + extra_hands,
            W3,
            H3,
            cells=ac.CELLS["oc-v3"],
        )
        self.assertTrue(r["ok"], r["issues"])

    def test_body_rules_still_apply(self):
        r = ac.judge(
            V3_FACES + [box(700, 40)],
            V3_HANDS + [box(100, 300)],
            W3,
            H3,
            cells=ac.CELLS["oc-v3"],
        )
        self.assertIn("back: face seen (head facing the wrong way)", r["issues"])
        self.assertIn("front: 3 hands", r["issues"])


class Query(unittest.TestCase):
    def test_parse_query(self):
        self.assertEqual(ac.parse_query("/check"), (("front", "left", "right"), "v2"))
        self.assertEqual(
            ac.parse_query("/check?face_views=front&layout=oc-v3"),
            (("front",), "oc-v3"),
        )
        self.assertEqual(
            ac.parse_query("/check?layout=oc-v3"), (("front", "left", "right"), "oc-v3")
        )
        with self.assertRaises(ValueError):
            ac.parse_query("/check?layout=nope")


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

    def test_v3_cells_are_cropped_in_two_dimensions(self):
        from PIL import Image

        sys.modules["ultralytics"] = types.SimpleNamespace(YOLO=FakeYOLO)
        FakeYOLO.loads, FakeYOLO.calls = [], []
        with tempfile.TemporaryDirectory() as d:
            img = os.path.join(d, "sheet.png")
            Image.new("RGB", (1000, 400)).save(img)
            faces, hands, w, h = ac.detect(img, d, cells=ac.CELLS["oc-v3"])
        self.assertEqual(len(FakeYOLO.calls), 24)
        self.assertEqual(FakeYOLO.calls[0][1], (200, 400))
        self.assertEqual(FakeYOLO.calls[8][1], (100, 100))
        self.assertEqual(
            faces[6], [810.0, 120.0, 830.0, 140.0, 0.9]
        )  # head-right starts at (800, 100)


class PoseModels(unittest.TestCase):
    """/pose answers 503 unless both DWPose model files are installed."""

    def call_pose(self, files):
        with tempfile.TemporaryDirectory() as d:
            for name in files:
                open(os.path.join(d, name), "wb").close()
            old = ac.POSE_MODELS
            ac.POSE_MODELS = d
            try:
                h = object.__new__(ac.AnatomyHandler)
                h.headers = {"Content-Length": "4"}
                sent = []
                h.send_json = lambda code, obj: sent.append((code, obj))
                h.pose()
            finally:
                ac.POSE_MODELS = old
        return sent

    def test_no_models_gives_503(self):
        sent = self.call_pose([])
        self.assertEqual(sent[0][0], 503)
        self.assertIn("not installed", sent[0][1]["error"])

    def test_only_the_detector_gives_503(self):
        self.assertEqual(self.call_pose(["yolox_l.onnx"])[0][0], 503)

    def test_only_the_pose_model_gives_503(self):
        self.assertEqual(self.call_pose(["dw-ll_ucoco_384.onnx"])[0][0], 503)

    def test_an_empty_body_is_still_a_400(self):
        h = object.__new__(ac.AnatomyHandler)
        h.headers = {"Content-Length": "0"}
        sent = []
        h.send_json = lambda code, obj: sent.append((code, obj))
        h.pose()
        self.assertEqual(sent[0][0], 400)


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
