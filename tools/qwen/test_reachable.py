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


def fake_dns(names):
    """A resolver that knows only these host names."""

    def dns(name):
        if name in names:
            return "10.0.0.1"
        raise OSError(name)

    return dns


class Reachable(unittest.TestCase):
    """A run inside a container on the OVMS network resolves "ovms" itself; it must still go through the queue (2026-10-18)."""

    def test_inside_the_network_the_queue_name_is_used(self):
        url = pipeline.reachable(
            "http://ovms:8000/v3",
            has=lambda name: name == "ovms-queue",
            dns=fake_dns({"ovms", "ovms-queue"}),
            ip_of=lambda name: self.fail("no docker inspect needed"),
        )
        self.assertEqual(url, "http://ovms-queue:8000/v3")

    def test_on_the_host_the_queue_container_ip_is_used(self):
        url = pipeline.reachable(
            "http://ovms:8000/v3",
            has=lambda name: name == "ovms-queue",
            dns=fake_dns(set()),
            ip_of=lambda name: {"ovms-queue": "172.16.1.18"}.get(name),
        )
        self.assertEqual(url, "http://172.16.1.18:8000/v3")

    def test_without_a_queue_a_resolving_host_is_kept(self):
        url = pipeline.reachable(
            "http://ovms:8000/v3",
            has=lambda name: False,
            dns=fake_dns({"ovms"}),
            ip_of=lambda name: self.fail("no docker inspect needed"),
        )
        self.assertEqual(url, "http://ovms:8000/v3")

    def test_a_public_host_is_left_alone(self):
        url = pipeline.reachable(
            "https://api.example.com/v1",
            has=lambda name: False,
            dns=fake_dns({"api.example.com"}),
            ip_of=lambda name: None,
        )
        self.assertEqual(url, "https://api.example.com/v1")


if __name__ == "__main__":
    unittest.main()
