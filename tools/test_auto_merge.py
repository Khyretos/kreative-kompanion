"""Tests for tools/auto_merge.py (CI-MERGE). Run: python3 -m unittest tools/test_auto_merge.py"""

import importlib.util
import json
import os
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "auto_merge", os.path.join(HERE, "auto_merge.py")
)
am = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(am)

REQUIRED = ["build / server", "build / web", "lint / megalinter"]
CFG = {
    "required": REQUIRED,
    "opt_out_label": "no-auto-merge",
    "own_context": "auto-merge /",
    "base": "main",
}
SHA = "a" * 40


def pr(**extra):
    d = {
        "number": 7,
        "state": "open",
        "merged": False,
        "draft": False,
        "title": "[X-01] something",
        "labels": [],
        "mergeable": True,
        "head": {"sha": SHA, "repo": {"full_name": "o/r"}},
        "base": {"ref": "main", "repo": {"full_name": "o/r"}},
    }
    d.update(extra)
    return d


def statuses(**state_by_name):
    """Commit statuses; keyword name 'build_server' -> context 'build / server (pull_request)'."""
    out = []
    for name, state in state_by_name.items():
        ctx = name.replace("__", " / ").replace("_", "-") + " (pull_request)"
        out.append({"context": ctx, "status": state})
    return out


GREEN = statuses(build__server="success", build__web="success", lint__megalinter="success")


class Skips(unittest.TestCase):
    def test_closed_and_merged_prs_are_skipped(self):
        for extra in ({"state": "closed"}, {"merged": True}):
            self.assertIsNotNone(am.skip_reason(pr(**extra), CFG))

    def test_draft_flag_and_wip_titles_are_skipped(self):
        self.assertIsNotNone(am.skip_reason(pr(draft=True), CFG))
        for title in ("WIP: x", "[WIP] x", "Draft: x", "draft: x", "wip x"):
            self.assertIsNotNone(am.skip_reason(pr(title=title), CFG), title)

    def test_title_that_only_contains_wip_is_not_a_draft(self):
        self.assertIsNone(am.skip_reason(pr(title="[X-02] wipe the cache"), CFG))

    def test_opt_out_label_is_skipped(self):
        self.assertIsNotNone(am.skip_reason(pr(labels=[{"name": "no-auto-merge"}]), CFG))
        self.assertIsNone(am.skip_reason(pr(labels=[{"name": "bug"}]), CFG))

    def test_fork_and_other_base_are_skipped(self):
        fork = pr(head={"sha": SHA, "repo": {"full_name": "x/r"}})
        self.assertIsNotNone(am.skip_reason(fork, CFG))
        other = pr(base={"ref": "dev", "repo": {"full_name": "o/r"}})
        self.assertIsNotNone(am.skip_reason(other, CFG))

    def test_plain_pr_is_not_skipped(self):
        self.assertIsNone(am.skip_reason(pr(), CFG))


class Checks(unittest.TestCase):
    def test_all_required_green(self):
        self.assertEqual(am.check_state(GREEN, CFG)[0], "ok")

    def test_own_context_is_ignored(self):
        own = GREEN + [{"context": "auto-merge / merge (pull_request)", "status": "pending"}]
        self.assertEqual(am.check_state(own, CFG)[0], "ok")

    def test_missing_required_check_waits(self):
        part = statuses(build__server="success", lint__megalinter="success")
        state, why = am.check_state(part, CFG)
        self.assertEqual(state, "wait")
        self.assertIn("build / web", why)

    def test_pending_check_waits(self):
        s = statuses(build__server="pending", build__web="success", lint__megalinter="success")
        self.assertEqual(am.check_state(s, CFG)[0], "wait")

    def test_failed_check_blocks_and_names_it(self):
        s = statuses(build__server="failure", build__web="success", lint__megalinter="success")
        state, why = am.check_state(s, CFG)
        self.assertEqual(state, "blocked")
        self.assertIn("build / server", why)

    def test_error_state_blocks(self):
        s = statuses(build__server="error", build__web="success", lint__megalinter="success")
        self.assertEqual(am.check_state(s, CFG)[0], "blocked")

    def test_failure_beats_pending(self):
        s = statuses(build__server="failure", build__web="pending")
        self.assertEqual(am.check_state(s, CFG)[0], "blocked")

    def test_extra_failing_check_blocks(self):
        s = GREEN + statuses(build__extra="failure")
        self.assertEqual(am.check_state(s, CFG)[0], "blocked")

    def test_no_statuses_waits(self):
        self.assertEqual(am.check_state([], CFG)[0], "wait")

    def test_only_the_newest_status_per_context_counts(self):
        s = [
            {"context": "build / server (pull_request)", "status": "failure"},
            {"context": "build / server (pull_request)", "status": "success"},
            {"context": "build / web (pull_request)", "status": "success"},
            {"context": "lint / megalinter (pull_request)", "status": "success"},
        ]
        self.assertEqual(am.check_state(s, CFG)[0], "ok")


class FakeApi:
    """Records calls; answers GET from a dict keyed by path suffix."""

    def __init__(self, pulls=None, stats=None, merge_status=200):
        self.pulls = pulls or {}
        self.stats = stats if stats is not None else GREEN
        self.merge_status = merge_status
        self.merged = []

    def pull(self, n):
        return self.pulls[n]

    def open_pulls(self):
        return list(self.pulls.values())

    def statuses(self, sha):
        return self.stats

    def merge(self, n, sha):
        self.merged.append((n, sha))
        return self.merge_status


def gate_returning(code, calls=None):
    def gate(n):
        if calls is not None:
            calls.append(n)
        return code

    return gate


class Process(unittest.TestCase):
    def test_merges_when_checks_and_gate_pass(self):
        api = FakeApi({7: pr()})
        out = am.process(7, api, CFG, gate_returning(0))
        self.assertEqual(out[0], "merged")
        self.assertEqual(api.merged, [(7, SHA)])

    def test_dry_run_does_not_merge(self):
        api = FakeApi({7: pr()})
        out = am.process(7, api, CFG, gate_returning(0), dry_run=True)
        self.assertEqual(out[0], "would merge")
        self.assertEqual(api.merged, [])

    def test_gate_is_not_run_before_checks_are_green(self):
        calls = []
        api = FakeApi({7: pr()}, stats=statuses(build__server="pending"))
        out = am.process(7, api, CFG, gate_returning(0, calls))
        self.assertEqual(out[0], "wait")
        self.assertEqual(calls, [])
        self.assertEqual(api.merged, [])

    def test_failed_check_never_merges(self):
        api = FakeApi({7: pr()}, stats=statuses(build__server="failure"))
        self.assertEqual(am.process(7, api, CFG, gate_returning(0))[0], "blocked")
        self.assertEqual(api.merged, [])

    def test_gate_blocking_findings_never_merge(self):
        api = FakeApi({7: pr()})
        out = am.process(7, api, CFG, gate_returning(1))
        self.assertEqual(out[0], "blocked")
        self.assertEqual(api.merged, [])

    def test_gate_pending_review_waits(self):
        api = FakeApi({7: pr()})
        out = am.process(7, api, CFG, gate_returning(2))
        self.assertEqual(out[0], "wait")
        self.assertEqual(api.merged, [])

    def test_unknown_gate_exit_code_is_not_a_pass(self):
        api = FakeApi({7: pr()})
        out = am.process(7, api, CFG, gate_returning(137))
        self.assertEqual(out[0], "blocked")
        self.assertEqual(api.merged, [])

    def test_conflicts_block(self):
        api = FakeApi({7: pr(mergeable=False)})
        out = am.process(7, api, CFG, gate_returning(0))
        self.assertEqual(out[0], "blocked")
        self.assertEqual(api.merged, [])

    def test_draft_and_opt_out_skip_before_anything_else(self):
        calls = []
        for extra in ({"draft": True}, {"labels": [{"name": "no-auto-merge"}]}):
            api = FakeApi({7: pr(**extra)})
            out = am.process(7, api, CFG, gate_returning(0, calls))
            self.assertEqual(out[0], "skip")
            self.assertEqual(api.merged, [])
        self.assertEqual(calls, [])

    def test_head_moved_during_the_gate_does_not_merge(self):
        class Moving(FakeApi):
            def __init__(self):
                super().__init__({7: pr()})
                self.n = 0

            def pull(self, n):
                self.n += 1
                p = pr()
                if self.n > 1:
                    p["head"] = {"sha": "b" * 40, "repo": {"full_name": "o/r"}}
                return p

        api = Moving()
        out = am.process(7, api, CFG, gate_returning(0))
        self.assertEqual(out[0], "wait")
        self.assertEqual(api.merged, [])

    def test_merge_conflict_409_waits_405_blocks(self):
        self.assertEqual(
            am.process(7, FakeApi({7: pr()}, merge_status=409), CFG, gate_returning(0))[0],
            "wait",
        )
        self.assertEqual(
            am.process(7, FakeApi({7: pr()}, merge_status=405), CFG, gate_returning(0))[0],
            "blocked",
        )

    def test_sweep_handles_every_open_pr(self):
        api = FakeApi({1: pr(number=1), 2: pr(number=2, draft=True), 3: pr(number=3)})
        results = am.sweep(api, CFG, gate_returning(0))
        self.assertEqual({n: r[0] for n, r in results.items()}, {1: "merged", 2: "skip", 3: "merged"})
        self.assertEqual([n for n, _ in api.merged], [1, 3])


class Config(unittest.TestCase):
    def test_missing_file_gives_defaults(self):
        cfg = am.load_config("/nonexistent/auto-merge.json")
        self.assertEqual(cfg["opt_out_label"], "no-auto-merge")
        self.assertEqual(cfg["base"], "main")
        self.assertTrue(cfg["required"])

    def test_file_overrides_defaults(self):
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump({"required": ["x / y"], "opt_out_label": "hold"}, f)
        try:
            cfg = am.load_config(f.name)
        finally:
            os.unlink(f.name)
        self.assertEqual(cfg["required"], ["x / y"])
        self.assertEqual(cfg["opt_out_label"], "hold")
        self.assertEqual(cfg["base"], "main")

    def test_shipped_config_lists_the_real_checks(self):
        cfg = am.load_config(os.path.join(HERE, "..", ".forgejo", "auto-merge.json"))
        for name in (
            "build / skills",
            "build / runner",
            "build / web",
            "build / server",
            "build / web-live",
            "lint / megalinter",
        ):
            self.assertIn(name, cfg["required"])


class Cli(unittest.TestCase):
    def test_merge_needs_a_token_unless_dry_run(self):
        env = {"FORGEJO_API": "http://x/api/v1", "REPO": "o/r"}
        self.assertEqual(am.main(["--pr", "7"], env), 1)

    def test_pr_must_be_a_number(self):
        with self.assertRaises(SystemExit):
            am.main(["--pr", "abc"], {})


if __name__ == "__main__":
    unittest.main()
