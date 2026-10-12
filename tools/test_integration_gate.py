"""Tests for tools/integration_gate.py (KG-09). Run: python3 -m unittest tools/test_integration_gate.py"""

import json
import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import integration_gate as g  # noqa: E402


def git(cwd, *a):
    return subprocess.run(
        ["git", *a], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout.strip()


def make_repo(tmp):
    git(tmp, "init", "-q", "-b", "main")
    git(tmp, "config", "user.email", "t@t")
    git(tmp, "config", "user.name", "t")
    with open(os.path.join(tmp, "ok"), "w") as f:
        f.write("1")
    git(tmp, "add", "-A")
    git(tmp, "commit", "-qm", "base")
    git(tmp, "tag", "gate-ok")


def merge_commit(tmp, n, title):
    git(tmp, "checkout", "-q", "-b", f"b{n}")
    with open(os.path.join(tmp, f"f{n}"), "w") as f:
        f.write(str(n))
    git(tmp, "add", "-A")
    git(tmp, "commit", "-qm", f"work {n}")
    git(tmp, "checkout", "-q", "main")
    git(
        tmp,
        "merge",
        "--no-ff",
        "-qm",
        f"Merge pull request '{title}' (#{n}) from b{n} into main",
        f"b{n}",
    )


class Gate(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory()
        self.tmp = self.d.name
        make_repo(self.tmp)

    def tearDown(self):
        self.d.cleanup()

    def test_counts_merges_since_the_last_good_gate(self):
        self.assertEqual(g.merges_since(self.tmp, "gate-ok"), [])
        merge_commit(self.tmp, 1, "[A-1] one")
        merge_commit(self.tmp, 2, "[A-2] two")
        m = g.merges_since(self.tmp, "gate-ok")
        self.assertEqual([x["pr"] for x in m], ["1", "2"])
        self.assertEqual(m[0]["title"], "[A-1] one")

    def test_waits_for_n_merges(self):
        merge_commit(self.tmp, 1, "[A-1] one")
        self.assertFalse(g.due(self.tmp, "gate-ok", 3))
        merge_commit(self.tmp, 2, "[A-2] two")
        merge_commit(self.tmp, 3, "[A-3] three")
        self.assertTrue(g.due(self.tmp, "gate-ok", 3))

    def test_passing_checks_move_the_tag_and_make_no_task(self):
        merge_commit(self.tmp, 1, "[A-1] one")
        cfg = {
            "every": 1,
            "checks": [
                {"name": "build", "cmd": "true"},
                {"name": "tests", "cmd": "echo 'tests: 12 passed'"},
            ],
        }
        out = g.run_gate(self.tmp, cfg, "gate-ok", baseline={"tests": 12})
        self.assertTrue(out["ok"])
        self.assertIsNone(out["task"])
        self.assertEqual(
            git(self.tmp, "rev-parse", "gate-ok"), git(self.tmp, "rev-parse", "HEAD")
        )

    def test_a_broken_merge_becomes_a_task_with_the_commit_range(self):
        merge_commit(self.tmp, 1, "[A-1] one")
        merge_commit(self.tmp, 2, "[A-2] breaks the build")
        head = git(self.tmp, "rev-parse", "--short", "HEAD")
        base = git(self.tmp, "rev-parse", "--short", "gate-ok")
        cfg = {
            "every": 1,
            "project": "kreative-kompanion",
            "checks": [{"name": "build", "cmd": "echo boom >&2; exit 3"}],
        }
        out = g.run_gate(self.tmp, cfg, "gate-ok", baseline={})
        self.assertFalse(out["ok"])
        t = out["task"]
        self.assertIn("Integration gate failed: build", t["title"])
        self.assertIn(f"{base}..{head}", t["description"])
        self.assertIn("[A-2] breaks the build", t["description"])
        self.assertIn("boom", t["description"])
        # the tag stays on the last good commit
        self.assertEqual(git(self.tmp, "rev-parse", "--short", "gate-ok"), base)

    def test_a_drop_below_the_baseline_fails(self):
        merge_commit(self.tmp, 1, "[A-1] one")
        cfg = {
            "every": 1,
            "checks": [{"name": "tests", "cmd": "echo 'tests: 10 passed'"}],
        }
        out = g.run_gate(self.tmp, cfg, "gate-ok", baseline={"tests": 12})
        self.assertFalse(out["ok"])
        self.assertIn("10 < baseline 12", out["task"]["description"])

    def test_the_task_is_an_import_file_for_kompanion(self):
        t = {"title": "Integration gate failed: build", "description": "x"}
        f = g.import_file(t, "kreative-kompanion", "2026-10-12")
        p = f["projects"][0]
        self.assertEqual(p["name"], "kreative-kompanion")
        self.assertEqual(p["tasks"][0]["state"], "queued")
        self.assertTrue(p["tasks"][0]["id"].startswith("gate-"))
        json.dumps(f)

    def test_long_output_is_cut(self):
        self.assertLessEqual(len(g.tail("x\n" * 5000)), 1500)


class ConfiguredChecks(unittest.TestCase):
    """The checks in .forgejo/integration-gate.json must carry a real exit code (2026-10-12: `| tail` hid it)."""

    def test_no_check_hides_its_exit_code(self):
        root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        with open(os.path.join(root, ".forgejo", "integration-gate.json")) as f:
            checks = json.load(f)["checks"]
        self.assertTrue(checks)
        for c in checks:
            self.assertNotIn("|", c["cmd"], c["name"])

    def test_web_check_installs_its_dependencies(self):
        root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        with open(os.path.join(root, ".forgejo", "integration-gate.json")) as f:
            web = [c for c in json.load(f)["checks"] if c["name"] == "web"][0]
        self.assertIn("npm ci", web["cmd"])


if __name__ == "__main__":
    unittest.main()
