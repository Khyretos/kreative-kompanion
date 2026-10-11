"""Tests for tools/video-metrics.py (STU-V1). Run: python3 -m unittest tools/test_video_metrics.py"""

import importlib.util
import os
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "vm", os.path.join(HERE, "video-metrics.py")
)
vm = importlib.util.module_from_spec(spec)
spec.loader.exec_module(vm)

SAMPLE = """frame:0    pts:0       pts_time:0
lavfi.signalstats.YMIN=16
lavfi.signalstats.YLOW=20
lavfi.signalstats.YAVG=100.5
lavfi.signalstats.YHIGH=200
lavfi.signalstats.UAVG=128
lavfi.signalstats.VAVG=130
lavfi.signalstats.SATAVG=10.5
frame:1    pts:512     pts_time:0.0416667
lavfi.signalstats.YLOW=22
lavfi.signalstats.YAVG=101
lavfi.signalstats.YHIGH=210
lavfi.signalstats.UAVG=127
lavfi.signalstats.VAVG=131
lavfi.signalstats.SATAVG=11
"""


def frames(ys, sat=10.0, con=100.0, u=128.0, v=128.0):
    return [
        {
            "YAVG": y,
            "SATAVG": sat,
            "YLOW": 50.0,
            "YHIGH": 50.0 + con,
            "UAVG": u,
            "VAVG": v,
        }
        for y in ys
    ]


def make(path, lum):
    subprocess.run(
        [
            "ffmpeg",
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=gray:s=160x96:d=2:r=24",
            "-vf",
            f"format=yuv420p,geq=lum='{lum}':cb=128:cr=128",
            "-c:v",
            "libx264",
            "-qp",
            "0",
            path,
        ],
        check=True,
    )


class Parse(unittest.TestCase):
    def test_parse_metadata(self):
        out = vm.parse_metadata(SAMPLE)
        self.assertEqual(len(out), 2)
        self.assertEqual(out[0]["YAVG"], 100.5)
        self.assertEqual(out[1]["YHIGH"], 210.0)
        self.assertEqual(out[0]["SATAVG"], 10.5)

    def test_parse_empty(self):
        self.assertEqual(vm.parse_metadata(""), [])


class Summary(unittest.TestCase):
    def test_steady(self):
        s = vm.summarize(frames([100.0] * 40), [10.0] * 40)
        self.assertEqual(s["frames"], 40)
        for k in ("drift_y", "drift_sat", "drift_con", "cast", "detail", "flicker"):
            self.assertAlmostEqual(s[k], 0.0, places=3, msg=k)

    def test_drift_y_head_tail_8(self):
        ys = [100.0] * 8 + [150.0] * 24 + [120.0] * 8
        s = vm.summarize(frames(ys), [10.0] * 40)
        self.assertAlmostEqual(s["drift_y"], 20.0, places=3)

    def test_short_clip_quarter(self):
        ys = [100.0] * 2 + [110.0] * 4 + [80.0] * 2
        s = vm.summarize(frames(ys), [10.0] * 8)
        self.assertAlmostEqual(s["drift_y"], -20.0, places=3)

    def test_cast_and_detail(self):
        f = frames([100.0] * 8, u=128.0) + frames([100.0] * 8, u=131.0, v=132.0)
        s = vm.summarize(f, [20.0] * 8 + [15.0] * 8)
        self.assertAlmostEqual(s["cast"], 5.0, places=3)
        self.assertAlmostEqual(s["detail"], -25.0, places=3)

    def test_flicker(self):
        s = vm.summarize(frames([100.0, 110.0] * 10), [10.0] * 20)
        self.assertAlmostEqual(s["flicker"], 10.0, places=3)

    def test_sat_zero_head(self):
        s = vm.summarize(frames([100.0] * 16, sat=0.0), [0.0] * 16)
        self.assertEqual(s["drift_sat"], 0.0)
        self.assertEqual(s["detail"], 0.0)


class EndToEnd(unittest.TestCase):
    def test_measure_ramp_and_steady(self):
        with tempfile.TemporaryDirectory() as d:
            ramp, flat = os.path.join(d, "ramp.mp4"), os.path.join(d, "flat.mp4")
            make(ramp, "80+N*2")
            make(flat, "120")
            r, f = vm.measure(ramp), vm.measure(flat)
            self.assertEqual(r["frames"], 48)
            self.assertGreater(r["drift_y"], 50.0)
            self.assertLess(abs(f["drift_y"]), 1.0)
            self.assertLess(f["flicker"], 0.5)
            out = subprocess.run(
                ["python3", os.path.join(HERE, "video-metrics.py"), "--json", flat],
                capture_output=True,
                text=True,
                timeout=60,
                check=False,
            )
            self.assertEqual(out.returncode, 0, out.stderr)
            self.assertIn('"frames": 48', out.stdout)
            line = subprocess.run(
                ["python3", os.path.join(HERE, "video-metrics.py"), ramp],
                capture_output=True,
                text=True,
                timeout=60,
                check=False,
            ).stdout
            self.assertIn(" frames=48 y=+", line)
            self.assertNotIn("++", line)
            bad = subprocess.run(
                [
                    "python3",
                    os.path.join(HERE, "video-metrics.py"),
                    os.path.join(d, "none.mp4"),
                ],
                capture_output=True,
                text=True,
                timeout=60,
                check=False,
            )
            self.assertEqual(bad.returncode, 1)


if __name__ == "__main__":
    unittest.main()
