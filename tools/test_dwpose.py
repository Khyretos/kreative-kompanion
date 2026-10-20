"""Tests for tools/anatomy/dwpose.py (STU-PE3: DWPose on the CPU, generated image -> OpenPose keypoints).
Written before the code. The ONNX sessions are fakes, so no model files are needed.
Run: python3 -m unittest tools/test_dwpose.py"""

import importlib.util
import os
import unittest

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "dw", os.path.join(HERE, "anatomy", "dwpose.py")
)
dw = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dw)


class Letterbox(unittest.TestCase):
    def test_pads_with_114_and_keeps_the_ratio(self):
        img = np.full((100, 200, 3), 7, dtype=np.uint8)
        t, ratio = dw.det_preprocess(img, (640, 640))
        self.assertEqual(t.shape, (1, 3, 640, 640))
        self.assertEqual(t.dtype, np.float32)
        self.assertAlmostEqual(ratio, 3.2)
        self.assertEqual(float(t[0, 0, 0, 0]), 7.0)  # top-left is image
        self.assertEqual(float(t[0, 0, 639, 0]), 114.0)  # below the image is padding
        self.assertEqual(float(t[0, 2, 319, 639]), 7.0)  # 320 rows of image, full width


class Detect(unittest.TestCase):
    def test_grid_decode(self):
        # one prediction per cell; strides 8, 16, 32 on 64 x 64 -> 64 + 16 + 4 cells
        n = 64 + 16 + 4
        out = np.zeros((1, n, 6), dtype=np.float32)
        out[0, 9, :4] = [0.5, 0.25, 0.0, np.log(2.0)]  # stride 8 cell (x=1, y=1)
        out[0, 64, :4] = [0.0, 0.0, np.log(3.0), 0.0]  # first stride 16 cell
        d = dw.det_grid_decode(out, (64, 64))
        np.testing.assert_allclose(
            d[0, 9, :4], [(1 + 0.5) * 8, (1 + 0.25) * 8, 8.0, 16.0], rtol=1e-6
        )
        np.testing.assert_allclose(d[0, 64, :4], [0.0, 0.0, 48.0, 16.0], rtol=1e-6)

    def test_nms_keeps_the_best_of_overlapping_boxes(self):
        boxes = np.array(
            [[0, 0, 10, 10], [1, 1, 11, 11], [50, 50, 60, 60]], dtype=np.float32
        )
        scores = np.array([0.6, 0.9, 0.5], dtype=np.float32)
        self.assertEqual(dw.nms(boxes, scores, 0.45), [1, 2])

    def test_people_boxes_from_raw_output(self):
        class FakeDet:
            def get_inputs(self):
                return [type("I", (), {"name": "images"})()]

            def run(self, _names, feed):
                t = feed["images"]
                n = sum((t.shape[2] // s) * (t.shape[3] // s) for s in (8, 16, 32))
                out = np.zeros((1, n, 85), dtype=np.float32)
                # stride 32 cell (x=5, y=5) in a 640 input: centre (176, 176), size 64 x 128 (log 2, log 4)
                i = 6400 + 1600 + 5 * 20 + 5
                out[0, i, :6] = [0.5, 0.5, np.log(2.0), np.log(4.0), 0.9, 0.9]
                out[0, i + 1, :6] = [
                    0.5,
                    0.5,
                    0.0,
                    0.0,
                    0.9,
                    0.1,
                ]  # person score 0.09: dropped
                return [out]

        img = np.zeros((320, 320, 3), dtype=np.uint8)  # ratio 2
        boxes = dw.detect_people(FakeDet(), img)
        np.testing.assert_allclose(boxes, [[72.0, 56.0, 104.0, 120.0]], rtol=1e-5)


class Pose(unittest.TestCase):
    def test_box_to_centre_and_scale(self):
        c, s = dw.box_to_cs([10, 20, 110, 70], padding=1.25)
        np.testing.assert_allclose(c, [60, 45])
        np.testing.assert_allclose(s, [125, 62.5])

    def test_fix_aspect_ratio(self):
        np.testing.assert_allclose(
            dw.fix_aspect(np.array([125.0, 62.5]), 288 / 384), [125.0, 125 / 0.75]
        )
        np.testing.assert_allclose(
            dw.fix_aspect(np.array([30.0, 100.0]), 0.75), [75.0, 100.0]
        )

    def test_simcc_decode(self):
        sx = np.zeros((1, 2, 576), dtype=np.float32)
        sy = np.zeros((1, 2, 768), dtype=np.float32)
        sx[0, 0, 100] = 0.8
        sy[0, 0, 300] = 0.6
        sx[0, 1, 10] = 0.2
        sy[0, 1, 20] = 0.4
        kp, sc = dw.simcc_decode(sx, sy, 2.0)
        np.testing.assert_allclose(kp[0], [[50, 150], [5, 10]])
        np.testing.assert_allclose(sc[0], [0.6, 0.2])

    def test_back_to_image_coordinates(self):
        kp = np.array([[[144.0, 192.0], [0.0, 0.0]]])
        out = dw.to_image(
            kp, (288, 384), np.array([60.0, 45.0]), np.array([150.0, 200.0])
        )
        np.testing.assert_allclose(out[0], [[60, 45], [-15, -55]])


class Chain(unittest.TestCase):
    def test_estimate_runs_one_box_per_call_and_stacks_people(self):
        class FakePose:
            def get_inputs(self):
                return [type("I", (), {"name": "input"})()]

            def run(self, _names, feed):
                x = feed["input"]
                self_shape = x.shape
                assert self_shape == (1, 3, 384, 288), self_shape
                sx = np.zeros((1, 133, 576), dtype=np.float32)
                sy = np.zeros((1, 133, 768), dtype=np.float32)
                sx[0, :, 288] = 0.9  # every point in the middle of the crop
                sy[0, :, 384] = 0.9
                return [sx, sy]

        img = np.zeros((200, 300, 3), dtype=np.uint8)
        kp, sc = dw.estimate(
            FakePose(),
            img,
            np.array([[10, 20, 110, 170], [150, 20, 250, 170]], dtype=np.float32),
        )
        self.assertEqual(kp.shape, (2, 133, 2))
        self.assertEqual(sc.shape, (2, 133))
        np.testing.assert_allclose(kp[0, 0], [60, 95], atol=1e-3)  # the middle of box 0
        np.testing.assert_allclose(kp[1, 0], [200, 95], atol=1e-3)
        j = dw.to_openpose(kp, sc, 300, 200)
        self.assertEqual(len(j["people"]), 2)


class OpenPose(unittest.TestCase):
    def test_wholebody_to_openpose_json(self):
        kp = np.zeros((1, 133, 2), dtype=np.float32)
        sc = np.full((1, 133), 0.9, dtype=np.float32)
        for i in range(133):
            kp[0, i] = [i, 1000 + i]
        sc[0, 6] = 0.2  # right shoulder low: the neck takes the lower score
        j = dw.to_openpose(kp, sc, 640, 480)
        self.assertEqual(j["canvas_width"], 640)
        self.assertEqual(j["canvas_height"], 480)
        p = j["people"][0]
        body = np.array(p["pose_keypoints_2d"]).reshape(18, 3)
        np.testing.assert_allclose(body[0], [0, 1000, 0.9], rtol=1e-6)  # nose
        np.testing.assert_allclose(
            body[1], [5.5, 1005.5, 0.2], rtol=1e-6
        )  # neck = middle of the shoulders
        np.testing.assert_allclose(
            body[2], [6, 1006, 0.2], rtol=1e-6
        )  # r_shoulder = coco 6
        np.testing.assert_allclose(
            body[5], [5, 1005, 0.9], rtol=1e-6
        )  # l_shoulder = coco 5
        np.testing.assert_allclose(
            body[4], [10, 1010, 0.9], rtol=1e-6
        )  # r_wrist = coco 10
        np.testing.assert_allclose(
            body[8], [12, 1012, 0.9], rtol=1e-6
        )  # r_hip = coco 12
        np.testing.assert_allclose(
            body[13], [15, 1015, 0.9], rtol=1e-6
        )  # l_ankle = coco 15
        np.testing.assert_allclose(
            body[14], [2, 1002, 0.9], rtol=1e-6
        )  # r_eye = coco 2
        np.testing.assert_allclose(
            body[17], [3, 1003, 0.9], rtol=1e-6
        )  # l_ear = coco 3
        left = np.array(p["hand_left_keypoints_2d"]).reshape(21, 3)
        right = np.array(p["hand_right_keypoints_2d"]).reshape(21, 3)
        np.testing.assert_allclose(left[0], [91, 1091, 0.9], rtol=1e-6)
        np.testing.assert_allclose(right[20], [132, 1132, 0.9], rtol=1e-6)
        self.assertTrue(all(isinstance(v, float) for v in p["pose_keypoints_2d"]))

    def test_no_people(self):
        j = dw.to_openpose(np.zeros((0, 133, 2)), np.zeros((0, 133)), 10, 20)
        self.assertEqual(j, {"canvas_width": 10, "canvas_height": 20, "people": []})


if __name__ == "__main__":
    unittest.main()
