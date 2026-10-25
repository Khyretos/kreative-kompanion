"""Tests for tools/motion/server.py request checks (STU-M2). The model is not loaded. Run: python3 tools/test_motion_server.py"""

import importlib.util
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "ms", os.path.join(HERE, "motion", "server.py")
)
ms = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ms)


class Check(unittest.TestCase):
    def test_defaults(self):
        self.assertEqual(
            ms.check_request({"prompt": " a person waves. "}), ("a person waves.", 0, 1)
        )

    def test_seconds_to_frames(self):
        self.assertEqual(
            ms.check_request({"prompt": "walk", "seconds": 3, "seed": 7}),
            ("walk", 60, 7),
        )
        self.assertEqual(ms.check_request({"prompt": "walk", "seconds": 9.8})[1], 196)
        self.assertEqual(ms.check_request({"prompt": "walk", "seconds": 1.1})[1], 20)

    def test_refusals(self):
        for body, msg in (
            ({}, "prompt"),
            ({"prompt": ""}, "prompt"),
            ({"prompt": "x" * 301}, "300"),
            ({"prompt": "a", "seconds": 12}, "seconds"),
            ({"prompt": "a", "seconds": True}, "seconds"),
            ({"prompt": "a", "seed": -1}, "seed"),
            ([], "object"),
        ):
            with self.assertRaises(ValueError) as e:
                ms.check_request(body)
            self.assertIn(msg, str(e.exception))

    def test_joint_names(self):
        self.assertEqual(len(ms.JOINT_NAMES), 22)
        self.assertEqual(ms.JOINT_NAMES[15], "head")


if __name__ == "__main__":
    unittest.main()
