"""RVW-01: tests for review_findings.py (no network, no model).
Run: python3 -m unittest tools/qwen/test_review_findings.py"""

import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import review_findings as rf

STATE = {
    "findings": [
        {
            "finding_id": "aaa",
            "state": "ACTIVE",
            "path": "server/src/api.rs",
            "line_start": 3,
            "line_end": 4,
            "body": "**Unhandled Panic**\n\nThe call can panic.",
        },
        {
            "finding_id": "bbb",
            "state": "ACTIVE",
            "path": "web/src/main.ts",
            "body": "**Missing Error Handling**\n\nNo catch.",
        },
        {
            "finding_id": "ccc",
            "state": "RESOLVED",
            "path": "web/src/x.ts",
            "body": "**Old**\n\ngone",
        },
    ],
    "last_run": {"head_sha": "abc123", "complete": True},
}
BODY = (
    "## PR Reviewer Guide\n\ntext\n<!-- pr-agent-review-state:v1\n"
    + json.dumps(STATE)
    + "\n-->\n"
)


class StateTest(unittest.TestCase):
    def test_state_from_body(self):
        self.assertEqual(rf.state_from_body(BODY), STATE)

    def test_state_from_body_without_marker(self):
        self.assertIsNone(rf.state_from_body("## PR Reviewer Guide\n\nno state"))

    def test_open_findings_only_active(self):
        self.assertEqual(
            [f["finding_id"] for f in rf.open_findings(STATE)], ["aaa", "bbb"]
        )

    def test_title(self):
        self.assertEqual(rf.title(STATE["findings"][0]), "Unhandled Panic")


class GateTest(unittest.TestCase):
    def test_not_triaged(self):
        self.assertEqual(
            dict(rf.blocking(rf.open_findings(STATE), {})),
            {"aaa": "not triaged", "bbb": "not triaged"},
        )

    def test_blocking_reasons(self):
        ledger = {
            "aaa": {"verdict": "bug", "reason": "real", "checked": True, "fixed": ""},
            "bbb": {
                "verdict": "false_positive",
                "reason": "caught in caller",
                "checked": False,
            },
        }
        self.assertEqual(
            dict(rf.blocking(rf.open_findings(STATE), ledger)),
            {"aaa": "bug not fixed yet", "bbb": "verdict not checked by Claude"},
        )

    def test_fixed_but_unchecked_blocks(self):
        ledger = {
            "aaa": {"verdict": "bug", "checked": False, "fixed": "deadbee"},
            "bbb": {"verdict": "false_positive", "checked": True},
        }
        self.assertEqual(
            rf.blocking(rf.open_findings(STATE), ledger),
            [("aaa", "verdict not checked by Claude")],
        )

    def test_clear(self):
        ledger = {
            "aaa": {
                "verdict": "bug",
                "reason": "real",
                "checked": True,
                "fixed": "deadbee",
            },
            "bbb": {
                "verdict": "false_positive",
                "reason": "caught in caller",
                "checked": True,
            },
        }
        self.assertEqual(rf.blocking(rf.open_findings(STATE), ledger), [])


class VerdictTest(unittest.TestCase):
    def test_parse_plain_json(self):
        self.assertEqual(
            rf.parse_verdict('{"verdict": "bug", "reason": "x is None"}'),
            {"verdict": "bug", "reason": "x is None"},
        )

    def test_parse_fenced_json_with_prose(self):
        text = 'Here:\n```json\n{"verdict": "false_positive", "reason": "handled by ?"}\n```'
        self.assertEqual(rf.parse_verdict(text)["verdict"], "false_positive")

    def test_parse_rejects_unknown_verdict(self):
        with self.assertRaises(ValueError):
            rf.parse_verdict('{"verdict": "maybe", "reason": "?"}')

    def test_parse_rejects_no_json(self):
        with self.assertRaises(ValueError):
            rf.parse_verdict("I think it is a bug")


class ExcerptTest(unittest.TestCase):
    def test_excerpt_numbers_lines_around_range(self):
        with tempfile.NamedTemporaryFile("w", suffix=".rs", delete=False) as f:
            f.write("".join(f"line{i}\n" for i in range(1, 101)))
        try:
            text = rf.excerpt(f.name, 50, 51, radius=2)
            self.assertEqual(
                text.splitlines(),
                [
                    "48: line48",
                    "49: line49",
                    "50: line50",
                    "51: line51",
                    "52: line52",
                    "53: line53",
                ],
            )
            whole = rf.excerpt(f.name, None, None, radius=2, cap=10)
            self.assertEqual(whole.splitlines()[0], "1: line1")
            self.assertEqual(len(whole.splitlines()), 10)
        finally:
            os.unlink(f.name)

    def test_locate_quoted_name(self):
        with tempfile.NamedTemporaryFile("w", suffix=".rs", delete=False) as f:
            f.write("fn a() {}\nfn set_role_x() {}\nfn set_role() {}\n")
        try:
            self.assertEqual(
                rf.locate(f.name, "The `set_role` function (line 898) ..."), 3
            )
            self.assertIsNone(rf.locate(f.name, "no names here"))
        finally:
            os.unlink(f.name)

    def test_excerpt_missing_file(self):
        self.assertEqual(
            rf.excerpt("/nonexistent/x.rs", 1, 2), "(file not in this checkout)"
        )


class TriageTest(unittest.TestCase):
    def test_triage_sends_body_and_code_and_records_unchecked(self):
        import types

        seen = []
        answers = iter(
            [
                '{"verdict": "bug", "reason": "unwrap on a db error"}',
                "no json",
                "still none",
            ]
        )
        fake = types.SimpleNamespace(
            ask=lambda system, user, max_tokens: (
                seen.append(user),
                (next(answers), 1.0, 5),
            )[1]
        )
        sys.modules["pipeline"] = fake
        try:
            with tempfile.TemporaryDirectory() as root:
                os.makedirs(os.path.join(root, "server/src"))
                with open(os.path.join(root, "server/src/api.rs"), "w") as f:
                    f.write("fn a() {}\nlet x = q.unwrap();\n")
                ledger = {}
                rf.triage({"number": 1}, rf.open_findings(STATE), ledger, root)
        finally:
            del sys.modules["pipeline"]
        self.assertIn("The call can panic.", seen[0])
        self.assertIn("2: let x = q.unwrap();", seen[0])
        self.assertEqual(
            ledger["aaa"],
            {
                "title": "Unhandled Panic",
                "path": "server/src/api.rs",
                "verdict": "bug",
                "reason": "unwrap on a db error",
                "by": "coder",
                "checked": False,
                "fixed": "",
            },
        )
        self.assertEqual(
            ledger["bbb"]["reason"], "unparsed model answer, Claude decides"
        )
        self.assertEqual(ledger["bbb"]["checked"], False)


class RepeatTest(unittest.TestCase):
    def test_repeat_of_checked_false_positive_keeps_verdict_without_the_model(self):
        import types

        sys.modules["pipeline"] = types.SimpleNamespace(
            ask=lambda *a, **k: self.fail("model called")
        )
        try:
            ledger = {
                "old": {
                    "title": "Missing Error Handling",
                    "path": "web/src/main.ts",
                    "verdict": "false_positive",
                    "reason": "caught in caller",
                    "by": "coder",
                    "checked": True,
                    "fixed": "",
                },
                "aaa": {
                    "title": "Unhandled Panic",
                    "path": "server/src/api.rs",
                    "verdict": "bug",
                    "reason": "real",
                    "by": "coder",
                    "checked": True,
                    "fixed": "deadbee",
                },
            }
            rf.triage({"number": 1}, rf.open_findings(STATE), ledger, "/nonexistent")
        finally:
            del sys.modules["pipeline"]
        self.assertEqual(ledger["bbb"]["by"], "repeat of old")
        self.assertTrue(ledger["bbb"]["checked"])
        self.assertEqual(rf.blocking(rf.open_findings(STATE), ledger), [])


class JobsTest(unittest.TestCase):
    def test_fix_jobs_only_checked_unfixed_bugs(self):
        ledger = {
            "aaa": {
                "verdict": "bug",
                "reason": "real: unwrap on a db error",
                "checked": True,
                "fixed": "",
            },
            "bbb": {"verdict": "bug", "reason": "real", "checked": False},
        }
        jobs = rf.fix_jobs(rf.open_findings(STATE), ledger)
        self.assertEqual(len(jobs), 1)
        j = jobs[0]
        self.assertEqual(j["name"], "fix-aaa")
        self.assertEqual(j["role"], "worker/rust")
        self.assertEqual(j["context"], ["server/src/api.rs"])
        self.assertEqual(j["out"], "server/src/api.rs")
        self.assertEqual(j["check"], "")
        self.assertIn("Unhandled Panic", j["prompt"])
        self.assertIn("real: unwrap on a db error", j["prompt"])

    def test_role_for(self):
        self.assertEqual(rf.role_for("server/src/a.rs"), "worker/rust")
        self.assertEqual(rf.role_for("web/src/a.ts"), "worker/web")
        self.assertEqual(rf.role_for("tools/x.py"), "worker")


if __name__ == "__main__":
    unittest.main()
