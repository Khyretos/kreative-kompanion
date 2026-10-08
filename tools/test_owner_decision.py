"""Tests for tools/owner_decision.py (GATE-02). Run: python3 -m unittest tools/test_owner_decision.py"""

import importlib.util
import json
import os
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "owner_decision", os.path.join(HERE, "owner_decision.py")
)
od = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(od)

OPTIONS = [
    {"name": "Accept until Tauri moves", "tradeoff": "dated ignore"},
    {"name": "Patch glib ourselves", "tradeoff": "fork"},
    {"name": "Drop the Linux desktop app", "tradeoff": "no tray"},
    {"name": "Leave it waiting", "tradeoff": "stays visible"},
]
REC = "Accept until Tauri moves"


def glib(**extra):
    d = {
        "id": "trivy-glib-GHSA-wrw7-89jp-8q8g",
        "linter": "REPOSITORY_TRIVY",
        "file": "desktop/src-tauri/Cargo.lock",
        "line": 1283,
        "match": "GHSA-wrw7-89jp-8q8g",
        "message": "glib 0.18.5",
        "options": [dict(o) for o in OPTIONS],
        "recommended": REC,
        "decided": None,
    }
    d.update(extra)
    return d


class ParseChoice(unittest.TestCase):
    def test_option_number(self):
        for body in (
            "accept option 2",
            "Accept Option 2",
            "option 2 please",
            "I pick 2.",
            "choose #2",
            "accept 2",
            "go with option 2, thanks",
        ):
            self.assertEqual(od.parse_choice(body, OPTIONS, REC), 1, body)

    def test_accept_alone_is_the_recommended_option(self):
        for body in ("accept", "Accept.", "accept the recommended option", "accept recommended"):
            self.assertEqual(od.parse_choice(body, OPTIONS, REC), 0, body)

    def test_option_name(self):
        self.assertEqual(od.parse_choice("drop the linux desktop app", OPTIONS, REC), 2)

    def test_out_of_range_questions_or_nothing(self):
        for body in ("option 7", "accept 0", "what does option 2 cost?", "hm, not sure yet", ""):
            self.assertIsNone(od.parse_choice(body, OPTIONS, REC), body)

    def test_quoted_lines_are_ignored(self):
        body = "> 1. **Accept until Tauri moves** (recommended)\n> option 3\n\noption 4"
        self.assertEqual(od.parse_choice(body, OPTIONS, REC), 3)


class ApplyLint(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp()

    def test_trivyignore_option_writes_dated_entry(self):
        d = glib()
        d["options"][0]["apply"] = {"trivyignore": "GHSA-wrw7-89jp-8q8g", "days": 92}
        jobs = od.apply_lint(d, 0, "khyretos", "2026-10-08", "https://x/c/1", self.root)
        self.assertEqual(jobs, [])
        text = open(os.path.join(self.root, ".trivyignore")).read()
        self.assertIn("GHSA-wrw7-89jp-8q8g exp:2027-01-08", text)
        self.assertIn(
            "# trivy-glib-GHSA-wrw7-89jp-8q8g: Accept until Tauri moves (khyretos, 2026-10-08",
            text,
        )
        self.assertEqual(
            d["decided"],
            "Accept until Tauri moves (khyretos, 2026-10-08, https://x/c/1)",
        )

    def test_trivyignore_keeps_existing_lines_and_is_idempotent(self):
        path = os.path.join(self.root, ".trivyignore")
        with open(path, "w") as f:
            f.write("# old\nCVE-1 exp:2027-01-01\n")
        d = glib()
        d["options"][0]["apply"] = {"trivyignore": "GHSA-wrw7-89jp-8q8g", "days": 92}
        od.apply_lint(d, 0, "khyretos", "2026-10-08", "u", self.root)
        od.apply_lint(d, 0, "khyretos", "2026-10-08", "u", self.root)
        text = open(path).read()
        self.assertTrue(text.startswith("# old\nCVE-1 exp:2027-01-01\n"))
        self.assertEqual(text.count("GHSA-wrw7-89jp-8q8g exp:"), 1)

    def test_keep_waiting_option(self):
        d = glib()
        d["options"][3]["keep_waiting"] = True
        self.assertEqual(od.apply_lint(d, 3, "khyretos", "2026-10-08", "u", self.root), [])
        self.assertTrue(d["keep_waiting"])
        self.assertTrue(d["decided"].startswith("Leave it waiting (khyretos"))
        self.assertFalse(os.path.exists(os.path.join(self.root, ".trivyignore")))

    def test_jobs_option_returns_its_pipeline_jobs(self):
        d = glib()
        job = {"name": "drop-desktop", "role": "worker/web", "prompt": "p", "out": "x", "check": "true"}
        d["options"][2]["jobs"] = [job]
        self.assertEqual(od.apply_lint(d, 2, "khyretos", "2026-10-08", "u", self.root), [job])
        self.assertTrue(d["decided"].startswith("Drop the Linux desktop app"))


class Verdicts(unittest.TestCase):
    LEDGER = {
        "ae97b8c11446": {"verdict": "false_positive", "checked": False, "fixed": "", "title": "A", "path": "a.py"},
        "dedbef95465d": {"verdict": "bug", "checked": False, "fixed": "", "title": "B", "path": "b.py"},
        "0123456789ab": {"verdict": "bug", "checked": True, "fixed": "abc", "title": "C", "path": "c.py"},
    }

    def test_accept_alone_takes_every_proposed_verdict(self):
        self.assertEqual(
            od.parse_verdicts("accept", self.LEDGER),
            {"ae97b8c11446": "false_positive", "dedbef95465d": "bug"},
        )

    def test_explicit_verdicts_by_id_prefix(self):
        body = "ae97b8 bug\ndedbef95 false positive\nunknown12 bug"
        self.assertEqual(
            od.parse_verdicts(body, self.LEDGER),
            {"ae97b8c11446": "bug", "dedbef95465d": "false_positive"},
        )

    def test_accept_some(self):
        self.assertEqual(
            od.parse_verdicts("accept dedbef", self.LEDGER), {"dedbef95465d": "bug"}
        )

    def test_short_prefix_and_chatter_ignored(self):
        self.assertEqual(od.parse_verdicts("ae9 bug", self.LEDGER), {})
        self.assertEqual(od.parse_verdicts("looks fine to me", self.LEDGER), {})

    def test_record_verdicts(self):
        ledger = json.loads(json.dumps(self.LEDGER))
        bugs = od.record_verdicts(
            ledger, {"ae97b8c11446": "false_positive", "dedbef95465d": "bug"}, "khyretos"
        )
        self.assertEqual(bugs, ["dedbef95465d"])
        for fid in ("ae97b8c11446", "dedbef95465d"):
            self.assertTrue(ledger[fid]["checked"])
            self.assertEqual(ledger[fid]["by"], "khyretos")


class Owner(unittest.TestCase):
    def test_only_the_repo_owner_counts(self):
        answers = {
            "API/issues/comments/9": {"user": {"login": "mallory"}, "body": "accept"},
            "API/issues/comments/10": {"user": {"login": "khyretos"}, "body": "accept"},
            "API": {"owner": {"login": "khyretos"}},
        }
        real = od.call
        od.call = lambda method, url, token, body=None: answers[url]
        try:
            self.assertIsNone(od.owner_comment("API", "t", 9))
            self.assertEqual(od.owner_comment("API", "t", 10), "accept")
        finally:
            od.call = real


class Checks(unittest.TestCase):
    def test_check_by_file_type(self):
        self.assertIn("py_compile", od.check_for("tools/x.py"))
        self.assertIn("cargo check", od.check_for("server/src/a.rs"))
        self.assertIn("node --check", od.check_for("web/app.js"))
        self.assertEqual(od.check_for("README.md"), "true")


class LintFlow(unittest.TestCase):
    """main() on an issue_comment event, with the API and git faked."""

    def setUp(self):
        self.root = tempfile.mkdtemp()
        os.makedirs(os.path.join(self.root, ".forgejo"))
        d = glib()
        d["options"][0]["apply"] = {"trivyignore": "GHSA-wrw7-89jp-8q8g", "days": 92}
        with open(os.path.join(self.root, od.DECISIONS), "w") as f:
            json.dump([d], f, indent=2)
        self.event = os.path.join(self.root, "event.json")
        self.calls, self.gits = [], []
        self.real = (od.call, od.git, os.getcwd())
        os.chdir(self.root)
        od.git = lambda *args: self.gits.append(args) or ""
        os.environ.update(API="API", TOKEN="job", PR_TOKEN="bot", GITHUB_EVENT_PATH=self.event)

    def tearDown(self):
        od.call, od.git, cwd = self.real
        os.chdir(cwd)

    def run_with(self, login, body):
        with open(self.event, "w") as f:
            json.dump(
                {
                    "comment": {"id": 5, "html_url": "https://git/c/5", "body": body, "user": {"login": login}},
                    "issue": {"number": 146, "title": "Lint decision: trivy-glib-GHSA-wrw7-89jp-8q8g"},
                    "repository": {"owner": {"login": "khyretos"}, "default_branch": "main"},
                },
                f,
            )

        def fake(method, url, token, data=None):
            self.calls.append((method, url, token, data))
            if url == "API/issues/comments/5":
                return {"user": {"login": login}, "body": body}
            if url == "API":
                return {"owner": {"login": "khyretos"}}
            if url == "API/pulls" and method == "POST":
                return {"number": 150, "html_url": "https://git/pulls/150"}
            return {}

        od.call = fake
        return od.main()

    def test_someone_else_is_ignored(self):
        self.assertEqual(self.run_with("mallory", "accept option 4"), 0)
        self.assertEqual([c[0] for c in self.calls], ["GET", "GET"])
        self.assertEqual(self.gits, [])
        self.assertIsNone(json.load(open(od.DECISIONS))[0]["decided"])

    def test_owner_pick_is_applied_recorded_and_closed(self):
        self.assertEqual(self.run_with("khyretos", "accept option 1"), 0)
        d = json.load(open(od.DECISIONS))[0]
        self.assertTrue(d["decided"].startswith("Accept until Tauri moves (khyretos, "))
        self.assertIn("GHSA-wrw7-89jp-8q8g exp:", open(".trivyignore").read())
        self.assertIn(("checkout", "-q", "-B", "decision/trivy-glib-GHSA-wrw7-89jp-8q8g"), self.gits)
        posted = [c for c in self.calls if c[0] == "POST"]
        pr = next(c for c in posted if c[1] == "API/pulls")
        self.assertEqual(pr[2], "bot")
        self.assertEqual(pr[3]["head"], "decision/trivy-glib-GHSA-wrw7-89jp-8q8g")
        self.assertIn("#146", pr[3]["body"])
        self.assertFalse([c for c in self.calls if "/merge" in c[1]])  # the owner merges it
        note = next(c for c in posted if c[1] == "API/issues/146/comments")
        self.assertIn("Accept until Tauri moves", note[3]["body"])
        self.assertIn("#150", note[3]["body"])
        self.assertIn(("PATCH", "API/issues/146", "job", {"state": "closed"}), self.calls)

    def test_owner_chatter_changes_nothing(self):
        self.assertEqual(self.run_with("khyretos", "what does option 2 cost?"), 0)
        self.assertEqual(self.gits, [])
        self.assertIsNone(json.load(open(od.DECISIONS))[0]["decided"])


class FindingsFlow(unittest.TestCase):
    """main() on a comment on PR #7 with PR-Agent findings, API, git and Coder faked."""

    STATE = {
        "findings": [
            {"finding_id": "ae97b8c11446", "state": "ACTIVE", "path": "a.py", "line_start": 1, "line_end": 2, "body": "**Hard-coded Secrets**\n\nx"},
            {"finding_id": "dedbef95465d", "state": "ACTIVE", "path": "b.py", "line_start": 1, "line_end": 2, "body": "**Missing Error Handling**\n\ny"},
            {"finding_id": "fedcba987654", "state": "ACTIVE", "path": "c.py", "line_start": 1, "line_end": 2, "body": "**Race**\n\nz"},
        ]
    }

    def setUp(self):
        self.root = tempfile.mkdtemp()
        os.makedirs(os.path.join(self.root, "docs", "review-findings"))
        os.makedirs(os.path.join(self.root, "tools", "qwen"))
        src = os.path.join(HERE, "qwen", "review_findings.py")
        with open(src) as f, open(os.path.join(self.root, "tools", "qwen", "review_findings.py"), "w") as g:
            g.write(f.read())
        for name in ("a.py", "b.py", "c.py"):
            with open(os.path.join(self.root, name), "w") as f:
                f.write("x = 1\n")
        self.ledger = os.path.join("docs", "review-findings", "pr-7.json")
        with open(os.path.join(self.root, self.ledger), "w") as f:
            json.dump(
                {
                    "ae97b8c11446": {"verdict": "false_positive", "checked": False, "fixed": "", "title": "Hard-coded Secrets", "path": "a.py", "reason": "r"},
                    "dedbef95465d": {"verdict": "bug", "checked": False, "fixed": "", "title": "Missing Error Handling", "path": "b.py", "reason": "r"},
                },
                f,
            )
        self.event = os.path.join(self.root, "event.json")
        self.calls, self.gits, self.jobs = [], [], []
        self.real = (od.call, od.git, od.run_pipeline, os.getcwd())
        os.chdir(self.root)
        od.git = lambda *args: self.gits.append(args) or "abc1234\n"
        od.run_pipeline = lambda jobs, log: self.jobs.extend(jobs)
        os.environ.update(API="API", TOKEN="job", PR_TOKEN="bot", GITHUB_EVENT_PATH=self.event)

    def tearDown(self):
        od.call, od.git, od.run_pipeline, cwd = self.real
        os.chdir(cwd)

    def run_with(self, body):
        with open(self.event, "w") as f:
            json.dump(
                {
                    "comment": {"id": 8, "html_url": "https://git/c/8", "body": body, "user": {"login": "khyretos"}},
                    "issue": {"number": 7, "title": "[X] a change", "pull_request": {"merged": False}},
                    "repository": {"owner": {"login": "khyretos"}, "default_branch": "main"},
                },
                f,
            )
        review = "<!-- pr-agent-review-state:v1\n" + json.dumps(self.STATE) + "\n-->"

        def fake(method, url, token, data=None):
            self.calls.append((method, url, token, data))
            if url == "API/issues/comments/8":
                return {"user": {"login": "khyretos"}, "body": body}
            if url == "API":
                return {"owner": {"login": "khyretos"}}
            if url.startswith("API/issues/7/comments") and method == "GET":
                return [{"user": {"login": "pr-agent"}, "body": review}]
            if url == "API/pulls" and method == "POST":
                return {"number": 151}
            return {}

        od.call = fake
        return od.main()

    def note(self):
        return next(c for c in self.calls if c[0] == "POST" and c[1] == "API/issues/7/comments")[3]["body"]

    def test_false_positive_only_needs_no_commit(self):
        self.assertEqual(self.run_with("ae97b8 false positive"), 0)
        self.assertEqual(self.jobs, [])
        self.assertFalse([g for g in self.gits if g[0] in ("commit", "push")])
        self.assertFalse([c for c in self.calls if c[1] == "API/pulls"])
        note = self.note()
        self.assertIn("ae97b8c11446", note)
        self.assertIn("false_positive", note)
        # still open: the bug Coder proposed and the untriaged one
        self.assertIn("fedcba987654", note)
        self.assertIn("'fedcba987654 bug'", note)

    def test_accept_fixes_bugs_in_a_new_pull_request(self):
        self.assertEqual(self.run_with("accept"), 0)
        self.assertEqual([j["out"] for j in self.jobs], ["b.py"])
        self.assertEqual(self.jobs[0]["check"], "python3 -m py_compile b.py")
        ledger = json.load(open(self.ledger))
        self.assertTrue(ledger["dedbef95465d"]["checked"])
        self.assertEqual(ledger["dedbef95465d"]["fixed"], "abc1234")
        self.assertEqual(ledger["ae97b8c11446"]["by"], "khyretos")
        self.assertNotIn("fedcba987654", {k for k, v in ledger.items() if v.get("checked")})
        self.assertIn(("push", "-q", "-f", "origin", "HEAD:refs/heads/findings/pr-7"), self.gits)
        pr = next(c for c in self.calls if c[1] == "API/pulls")
        self.assertEqual(pr[2], "bot")
        self.assertIn("#7", pr[3]["body"])
        self.assertIn("#151", self.note())


if __name__ == "__main__":
    unittest.main()
