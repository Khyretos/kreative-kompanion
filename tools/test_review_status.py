"""Tests for tools/review_status.py and review_status_io.py (GATE-IN-CI). Run: python3 -m unittest tools/test_review_status.py"""

import importlib.util
import json
import os
import unittest
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "review_status", os.path.join(HERE, "review_status.py")
)
rs = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rs)

rio = None  # review_status_io, loaded by Posting.setUpClass

SHA = "b" * 40


class Api:
    """Fake Forgejo client: open PRs in, posted statuses out."""

    def __init__(self, pulls):
        self.pulls = pulls
        self.posted = []

    def open_pulls(self):
        return self.pulls

    def post_status(self, sha, state, description):
        self.posted.append((sha, state, description))


def pull(n=7, sha=SHA, **extra):
    d = {"number": n, "state": "open", "head": {"sha": sha}}
    d.update(extra)
    return d


class StatusFor(unittest.TestCase):
    def test_exit_codes_map_to_states(self):
        self.assertEqual(rs.status_for(0)[0], "success")
        self.assertEqual(rs.status_for(2)[0], "pending")
        self.assertEqual(rs.status_for(1)[0], "failure")

    def test_a_gate_that_could_not_run_is_an_error_not_a_pass(self):
        for code in (3, 127, -9):
            self.assertEqual(rs.status_for(code)[0], "error", code)

    def test_every_state_has_a_description(self):
        for code in (0, 1, 2, 3):
            self.assertTrue(rs.status_for(code)[1], code)


class Sweep(unittest.TestCase):
    def test_posts_the_gate_result_on_each_head(self):
        api = Api([pull(7, "a" * 40), pull(8, "c" * 40)])
        codes = {7: 0, 8: 1}
        out = rs.sweep(api, lambda n: codes[n])
        self.assertEqual(out, {7: "success", 8: "failure"})
        self.assertEqual(
            [(s, st) for s, st, _ in api.posted],
            [("a" * 40, "success"), ("c" * 40, "failure")],
        )

    def test_pending_review_posts_pending(self):
        api = Api([pull()])
        rs.sweep(api, lambda n: 2)
        self.assertEqual(api.posted[0][1], "pending")

    def test_one_crashing_gate_does_not_stop_the_other_prs(self):
        def gate(n):
            if n == 7:
                raise OSError("boom")
            return 0

        api = Api([pull(7), pull(8, "c" * 40)])
        out = rs.sweep(api, gate)
        self.assertEqual(out[7], "error")
        self.assertEqual(out[8], "success")
        self.assertEqual(len(api.posted), 2)

    def test_draft_titles_are_not_gated(self):
        api = Api([pull(title="WIP: x")])
        self.assertEqual(rs.sweep(api, lambda n: 0), {})
        self.assertEqual(api.posted, [])

    def test_description_is_short_enough_for_forgejo(self):
        for code in (0, 1, 2, 3):
            self.assertLess(len(rs.status_for(code)[1]), 140)


class FakeResponse:
    status = 201

    def read(self):
        return b"{}"

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False


class Posting(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        global rio
        spec = importlib.util.spec_from_file_location(
            "review_status_io", os.path.join(HERE, "review_status_io.py")
        )
        rio = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(rio)

    def test_post_status_sends_state_context_and_token(self):
        seen = []

        def fake_urlopen(req, timeout=None):
            seen.append(req)
            return FakeResponse()

        rio.urllib.request.urlopen = fake_urlopen
        try:
            client = rio.StatusClient("https://f/api/v1", "o/r", "tok")
            client.post_status(SHA, "success", "all findings handled")
        finally:
            rio.urllib.request.urlopen = urllib.request.urlopen
        req = seen[0]
        self.assertEqual(req.get_method(), "POST")
        self.assertTrue(req.full_url.endswith("/repos/o/r/statuses/" + SHA))
        self.assertEqual(req.get_header("Authorization"), "token tok")
        body = json.loads(req.data)
        self.assertEqual(body["state"], "success")
        self.assertEqual(body["context"], rio.CONTEXT)
        self.assertEqual(body["description"], "all findings handled")

    def test_context_is_the_name_ci_requires(self):
        self.assertEqual(rio.CONTEXT, "review-gate / gate")

    def test_main_without_api_settings_fails(self):
        self.assertEqual(rio.main([], env={}), 1)


if __name__ == "__main__":
    unittest.main()
