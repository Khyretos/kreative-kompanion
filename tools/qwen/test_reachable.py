"""The Coder pipeline reaches OVMS through its queue container when there is one (2026-10-14)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import pipeline  # noqa: E402


class ResolveContainer(unittest.TestCase):
    def test_prefers_the_queue_container(self):
        self.assertEqual(
            pipeline.resolve_container("ovms", lambda name: name == "ovms-queue"),
            "ovms-queue",
        )

    def test_falls_back_to_the_container_itself(self):
        self.assertEqual(pipeline.resolve_container("ovms", lambda name: False), "ovms")

    def test_a_queue_name_is_not_queued_again(self):
        self.assertEqual(
            pipeline.resolve_container("ovms-queue", lambda name: True), "ovms-queue"
        )


if __name__ == "__main__":
    unittest.main()
