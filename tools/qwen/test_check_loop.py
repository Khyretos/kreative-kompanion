"""Run: python3 tools/qwen/test_check_loop.py  (a failing check always gets a repair round fed with its errors)."""

import io
import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pipeline  # noqa: E402


class FixRounds(unittest.TestCase):
    def setUp(self):
        self.saved = (pipeline.run_check, pipeline.patch_job)

    def tearDown(self):
        pipeline.run_check, pipeline.patch_job = self.saved

    def run_loop(self, errors, out="tools/qwen/pipeline.py"):
        calls, results = [], iter([(False, errors), (True, "")])
        pipeline.run_check = lambda cmd: next(results)
        pipeline.patch_job = lambda job, log: calls.append(job)
        log = io.StringIO()
        pipeline.check_loop({"name": "j", "out": out, "check": "x", "role": "worker/python"}, log)
        return calls, json.loads(log.getvalue())

    def test_errors_that_do_not_name_the_file_still_get_a_repair_round(self):
        calls, rec = self.run_loop("EE\nAttributeError: module 'pipeline' has no attribute 'trace_call'")
        self.assertEqual(len(calls), 1)
        self.assertIn("no attribute 'trace_call'", calls[0]["prompt"])
        self.assertTrue(rec["check_passed"])
        self.assertEqual(rec["check_attempts"], 2)

    def test_errors_that_name_the_file_are_still_filtered_to_it(self):
        errors = "src/other.rs:1: error: boom\n\nsrc/studio/x.rs:5: error: bad"
        calls, _ = self.run_loop(errors, out="src/studio/x.rs")
        self.assertEqual(len(calls), 1)
        self.assertIn("x.rs:5", calls[0]["prompt"])
        self.assertNotIn("other.rs", calls[0]["prompt"])


if __name__ == "__main__":
    unittest.main()
