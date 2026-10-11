"""Tests for tools/video-compare.py (STU-V1). Run: python3 -m unittest tools/test_video_compare.py"""

import importlib.util
import os
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "vc", os.path.join(HERE, "video-compare.py")
)
vc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(vc)

LAB = ":x=8:y=8:fontsize=18:fontcolor=white:box=1:boxcolor=black@0.6"


def probe(path, entry):
    return subprocess.run(
        [
            "ffprobe",
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            f"stream={entry}",
            "-of",
            "csv=p=0",
            path,
        ],
        capture_output=True,
        text=True,
        check=False,
    ).stdout.strip()


class Graph(unittest.TestCase):
    def test_two(self):
        g = vc.filter_graph(["base", "cpu"], 416, 240)
        self.assertEqual(
            g,
            "[0:v]scale=416:240,setsar=1,drawtext=text='base'" + LAB + "[v0];"
            "[1:v]scale=416:240,setsar=1,drawtext=text='cpu'" + LAB + "[v1];"
            "[v0][v1]hstack=inputs=2",
        )

    def test_label_clean(self):
        self.assertEqual(vc.clean_label("cfg:4 'x' 30%"), "cfg4 x 30")
        self.assertEqual(vc.clean_label("steps-30_a.b"), "steps-30_a.b")

    def test_cmd(self):
        c = vc.build_cmd(["a.mp4", "b.mp4"], ["a", "b"], "o.mp4", 416, 240)
        self.assertEqual(c[:4], ["ffmpeg", "-v", "error", "-y"])
        self.assertEqual(c[4:8], ["-i", "a.mp4", "-i", "b.mp4"])
        self.assertEqual(c[8], "-filter_complex")
        self.assertEqual(c[-1], "o.mp4")
        self.assertIn("-crf", c)

    def test_parse_arg(self):
        self.assertEqual(
            vc.parse_item("x/walk-base.mp4"), ("x/walk-base.mp4", "walk-base")
        )
        self.assertEqual(vc.parse_item("x/a.mp4=old graph"), ("x/a.mp4", "old graph"))


class EndToEnd(unittest.TestCase):
    def test_run(self):
        with tempfile.TemporaryDirectory() as d:
            ins = []
            for n in ("a", "b"):
                p = os.path.join(d, n + ".mp4")
                subprocess.run(
                    [
                        "ffmpeg",
                        "-v",
                        "error",
                        "-y",
                        "-f",
                        "lavfi",
                        "-i",
                        "testsrc=s=320x180:d=1:r=24",
                        "-pix_fmt",
                        "yuv420p",
                        p,
                    ],
                    check=True,
                )
                ins.append(p)
            out, sheet = os.path.join(d, "o.mp4"), os.path.join(d, "o.png")
            r = subprocess.run(
                [
                    "python3",
                    os.path.join(HERE, "video-compare.py"),
                    out,
                    ins[0],
                    ins[1] + "=two",
                    "--size",
                    "160x96",
                    "--sheet",
                    sheet,
                ],
                capture_output=True,
                text=True,
                timeout=120,
                check=False,
            )
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual(probe(out, "width,height"), "320,96")
            self.assertEqual(probe(sheet, "width,height"), "320,288")
            bad = subprocess.run(
                [
                    "python3",
                    os.path.join(HERE, "video-compare.py"),
                    out,
                    os.path.join(d, "none.mp4"),
                    ins[0],
                ],
                capture_output=True,
                text=True,
                timeout=60,
                check=False,
            )
            self.assertEqual(bad.returncode, 1)
            self.assertIn("error:", bad.stderr)


if __name__ == "__main__":
    unittest.main()
