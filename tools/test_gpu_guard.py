"""Tests for tools/gpu-guard/gpu_guard.py (STU-C4b). Run: python3 -m unittest tools/test_gpu_guard.py"""

import importlib.util
import json
import os
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
PATH = os.path.join(HERE, "gpu-guard", "gpu_guard.py")
spec = importlib.util.spec_from_file_location("gg", PATH)
gg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gg)

MIB = 1024 * 1024


def sample(free=8000, junction=60, mem=70):
    return {
        "total_mib": 16304,
        "used_mib": 16304 - free,
        "free_mib": free,
        "edge_c": junction - 2,
        "junction_c": junction,
        "mem_c": mem,
        "power_w": 200,
    }


def fake_device(
    root, used_mib=1000, total_mib=16304, temps=(43000, 44000, 65000), power=33000000
):
    dev = os.path.join(root, "device")
    hw = os.path.join(dev, "hwmon", "hwmon3")
    os.makedirs(hw)
    with open(os.path.join(dev, "mem_info_vram_used"), "w") as f:
        f.write(f"{used_mib * MIB}\n")
    with open(os.path.join(dev, "mem_info_vram_total"), "w") as f:
        f.write(f"{total_mib * MIB}\n")
    for i, (label, value) in enumerate(
        zip(("edge", "junction", "mem"), temps), start=1
    ):
        with open(os.path.join(hw, f"temp{i}_label"), "w") as f:
            f.write(label + "\n")
        with open(os.path.join(hw, f"temp{i}_input"), "w") as f:
            f.write(f"{value}\n")
    with open(os.path.join(hw, "power1_average"), "w") as f:
        f.write(f"{power}\n")
    return dev


class ReadSample(unittest.TestCase):
    def test_reads_vram_temps_and_power_from_sysfs(self):
        with tempfile.TemporaryDirectory() as d:
            dev = fake_device(d, used_mib=1000)
            s = gg.read_sample(dev)
        self.assertEqual(s["total_mib"], 16304)
        self.assertEqual(s["used_mib"], 1000)
        self.assertEqual(s["free_mib"], 15304)
        self.assertEqual(s["edge_c"], 43)
        self.assertEqual(s["junction_c"], 44)
        self.assertEqual(s["mem_c"], 65)
        self.assertEqual(s["power_w"], 33)

    def test_finds_temps_by_label_not_by_number(self):
        with tempfile.TemporaryDirectory() as d:
            dev = fake_device(d)
            hw = os.path.join(dev, "hwmon", "hwmon3")
            # swap labels 1 and 3: mem is now temp1
            with open(os.path.join(hw, "temp1_label"), "w") as f:
                f.write("mem\n")
            with open(os.path.join(hw, "temp3_label"), "w") as f:
                f.write("edge\n")
            s = gg.read_sample(dev)
        self.assertEqual(s["mem_c"], 43)
        self.assertEqual(s["edge_c"], 65)


class Decide(unittest.TestCase):
    def setUp(self):
        self.g = gg.Guard(gg.Limits())

    def test_defaults_match_the_plan(self):
        lim = gg.Limits()
        self.assertEqual(lim.min_free_mib, 1024)
        self.assertEqual(lim.low_secs, 6)
        self.assertEqual(lim.junction_c, 100)
        self.assertEqual(lim.mem_c, 100)

    def test_healthy_never_cancels(self):
        for t in range(0, 120, 2):
            self.assertIsNone(self.g.step(sample(), t, busy=True))

    def test_short_dip_is_tolerated(self):
        self.assertIsNone(self.g.step(sample(free=500), 0, busy=True))
        self.assertIsNone(self.g.step(sample(free=500), 4, busy=True))
        self.assertIsNone(self.g.step(sample(free=4000), 6, busy=True))
        self.assertIsNone(self.g.step(sample(free=500), 8, busy=True))
        self.assertIsNone(self.g.step(sample(free=500), 12, busy=True))

    def test_low_vram_for_six_seconds_cancels_with_reason(self):
        self.assertIsNone(self.g.step(sample(free=800), 0, busy=True))
        self.assertIsNone(self.g.step(sample(free=800), 2, busy=True))
        self.assertIsNone(self.g.step(sample(free=800), 4, busy=True))
        r = self.g.step(sample(free=800), 6, busy=True)
        self.assertIsNotNone(r)
        self.assertIn("800", r)
        self.assertIn("VRAM", r)

    def test_hot_junction_cancels(self):
        r = None
        for t in range(0, 20, 2):
            r = r or self.g.step(sample(junction=101), t, busy=True)
        self.assertIsNotNone(r)
        self.assertIn("junction", r)

    def test_hot_memory_cancels(self):
        r = None
        for t in range(0, 20, 2):
            r = r or self.g.step(sample(mem=100), t, busy=True)
        self.assertIsNotNone(r)
        self.assertIn("mem", r)

    def test_idle_comfyui_is_never_cancelled(self):
        for t in range(0, 60, 2):
            self.assertIsNone(
                self.g.step(sample(free=100, junction=105), t, busy=False)
            )

    def test_going_idle_resets_the_timer(self):
        self.g.step(sample(free=800), 0, busy=True)
        self.g.step(sample(free=800), 4, busy=True)
        self.g.step(sample(free=800), 5, busy=False)
        self.assertIsNone(self.g.step(sample(free=800), 6, busy=True))
        self.assertIsNone(self.g.step(sample(free=800), 10, busy=True))
        self.assertIsNotNone(self.g.step(sample(free=800), 12, busy=True))

    def test_cooldown_after_a_cancel(self):
        for t in range(0, 8, 2):
            self.g.step(sample(free=800), t, busy=True)
        for t in range(8, 36, 2):
            self.assertIsNone(self.g.step(sample(free=800), t, busy=True), t)
        r = None
        for t in range(36, 50, 2):
            r = r or self.g.step(sample(free=800), t, busy=True)
        self.assertIsNotNone(r)


class Cancel(unittest.TestCase):
    def test_clears_the_queue_then_interrupts(self):
        calls = []
        gg.cancel("http://x:8188/", post=lambda url, body: calls.append((url, body)))
        self.assertEqual(calls[0][0], "http://x:8188/queue")
        self.assertEqual(json.loads(calls[0][1]), {"clear": True})
        self.assertEqual(calls[1][0], "http://x:8188/interrupt")
        self.assertEqual(len(calls), 2)

    def test_a_failing_queue_call_still_interrupts(self):
        calls = []

        def post(url, body):
            calls.append(url)
            if url.endswith("/queue"):
                raise OSError("down")

        gg.cancel("http://x:8188", post=post)
        self.assertEqual(calls, ["http://x:8188/queue", "http://x:8188/interrupt"])


class Busy(unittest.TestCase):
    def test_busy_when_something_runs(self):
        self.assertTrue(gg.is_busy({"queue_running": [[1, "id"]], "queue_pending": []}))

    def test_not_busy_when_empty_or_unreachable(self):
        self.assertFalse(gg.is_busy({"queue_running": [], "queue_pending": [[2, "x"]]}))
        self.assertFalse(gg.is_busy(None))


class NeverKills(unittest.TestCase):
    def test_source_has_no_kill_or_restart(self):
        with open(PATH) as f:
            src = f.read()
        for bad in ("os.kill", "signal.", "pkill", "killall", "docker ", "subprocess"):
            self.assertNotIn(bad, src)


if __name__ == "__main__":
    unittest.main()
