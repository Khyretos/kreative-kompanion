"""Tests for tools/lint_gate.py (LINT-01). Run: python3 -m unittest tools/test_lint_gate.py"""

import importlib.util
import os
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "lint_gate", os.path.join(HERE, "lint_gate.py")
)
lint_gate = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(lint_gate)

GLIB = {
    "id": "trivy-glib",
    "linter": "REPOSITORY_TRIVY",
    "file": "desktop/src-tauri/Cargo.lock",
    "line": 1283,
    "match": "GHSA-wrw7-89jp-8q8g",
    "message": "glib 0.18.5",
    "why": "Tauri pins it.",
    "options": [
        {"name": "Accept", "tradeoff": "dated ignore"},
        {"name": "Wait", "tradeoff": "stays visible"},
    ],
    "recommended": "Accept",
    "decided": None,
}
TRIVY_ONE = "Total: 1 (UNKNOWN: 0, LOW: 0, MEDIUM: 1)\n| glib | GHSA-wrw7-89jp-8q8g | MEDIUM |\n"


def linter(name, status="error", errors=1):
    return {
        "name": name,
        "status": status,
        "total_number_errors": errors,
        "is_active": True,
    }


class Gate(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp()
        os.makedirs(os.path.join(self.dir, "linters_logs"))

    def log(self, name, text):
        with open(
            os.path.join(self.dir, "linters_logs", name + "-ERROR.log"), "w"
        ) as f:
            f.write(text)

    def test_clean_report_passes(self):
        report = {"linters": [linter("PYTHON_RUFF", "success", 0)]}
        self.assertTrue(lint_gate.gate(report, [], self.dir)[1])

    def test_any_problem_blocks(self):
        self.log("PYTHON_RUFF", "E741 Ambiguous variable name")
        report = {"linters": [linter("PYTHON_RUFF", errors=3)]}
        messages, ok = lint_gate.gate(report, [GLIB], self.dir)
        self.assertFalse(ok)
        self.assertIn("BLOCKING PYTHON_RUFF: 3 problem(s)", messages[0])

    def test_waiting_decision_does_not_block(self):
        self.log("REPOSITORY_TRIVY", TRIVY_ONE)
        report = {"linters": [linter("REPOSITORY_TRIVY")]}
        messages, ok = lint_gate.gate(report, [GLIB], self.dir)
        self.assertTrue(ok)
        self.assertEqual(
            messages,
            [
                "waiting trivy-glib: REPOSITORY_TRIVY desktop/src-tauri/Cargo.lock:1283 (issue)"
            ],
        )

    def test_more_trivy_findings_than_decisions_block(self):
        self.log(
            "REPOSITORY_TRIVY",
            TRIVY_ONE + "Failures: 1 (UNKNOWN: 0, LOW: 1)\nDS-0026\n",
        )
        report = {"linters": [linter("REPOSITORY_TRIVY")]}
        messages, ok = lint_gate.gate(report, [GLIB], self.dir)
        self.assertFalse(ok)
        self.assertIn("BLOCKING REPOSITORY_TRIVY: 2 problem(s), 1 waiting", messages[0])

    def test_decided_item_blocks_again(self):
        self.log("REPOSITORY_TRIVY", TRIVY_ONE)
        report = {"linters": [linter("REPOSITORY_TRIVY")]}
        self.assertFalse(
            lint_gate.gate(report, [dict(GLIB, decided="Accept")], self.dir)[1]
        )

    def test_fixed_item_is_reported_as_resolved(self):
        report = {"linters": [linter("REPOSITORY_TRIVY", "success", 0)]}
        messages, ok = lint_gate.gate(report, [GLIB], self.dir)
        self.assertTrue(ok)
        self.assertIn("resolved trivy-glib", messages[0])

    def test_issue_body_names_everything(self):
        body = lint_gate.issue_body(GLIB)
        for part in (
            "Lint report",
            "REPOSITORY_TRIVY",
            "desktop/src-tauri/Cargo.lock:1283",
            "glib 0.18.5",
            "**Accept** (recommended)",
            "**Wait**:",
        ):
            self.assertIn(part, body)

    def test_missing_report_fails(self):
        os.environ["REPORTS"] = self.dir
        try:
            self.assertEqual(lint_gate.main(), 1)
        finally:
            del os.environ["REPORTS"]


if __name__ == "__main__":
    unittest.main()
