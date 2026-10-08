#!/usr/bin/env python3
"""LINT-01: the lint job's merge gate. Runs after MegaLinter in the lint workflow.

Fails when any linter reports a problem, except problems that wait for Kees's decision
(.forgejo/lint-decisions.json). Each waiting item gets its own issue, which Forgejo mails
to Kees as "Kompas Linter": linter, file:line, the message and the options with one
recommended. Only that item waits; every other problem blocks the merge.

Usage (in the MegaLinter container, repo root): python3 tools/lint_gate.py
Env: API (repo API URL), TOKEN (job token) to open the decision issues; without them
the gate only checks. REPORTS (default megalinter-reports).
"""

import glob
import json
import os
import re
import sys
import urllib.parse
import urllib.request

DECISIONS = ".forgejo/lint-decisions.json"


def log_of(reports, linter):
    """The linter's log text without colour codes."""
    hits = glob.glob(os.path.join(reports, "linters_logs", linter + "-*.log"))
    text = open(hits[0], errors="replace").read() if hits else ""
    return re.sub(r"\x1b\[[0-9;]*m", "", text)


def problems(linter, report_errors, log):
    """How many problems a failing linter found. MegaLinter counts trivy as one
    problem whatever it found, so trivy's own totals are added up instead."""
    if linter == "REPOSITORY_TRIVY":
        found = re.findall(r"^(?:Total|Failures): (\d+)", log, re.M)
        return sum(int(n) for n in found) or 1
    return report_errors or 1


def gate(report, decisions, reports):
    """(messages, ok): one message per blocking linter or stale decision."""
    messages, ok = [], True
    waiting = [d for d in decisions if not d.get("decided")]
    for linter in report.get("linters", []):
        name = linter.get("name", "")
        if not linter.get("is_active", True) or linter.get("status") == "success":
            continue
        log = log_of(reports, name)
        mine = [d for d in waiting if d["linter"] == name]
        found = problems(name, linter.get("total_number_errors"), log)
        covered = [d for d in mine if d["match"] in log]
        if found > len(covered):
            ok = False
            messages.append(
                f"BLOCKING {name}: {found} problem(s), {len(covered)} waiting for a decision"
            )
        for d in covered:
            messages.append(
                f"waiting {d['id']}: {name} {d['file']}:{d['line']} (issue)"
            )
    for d in waiting:
        fails = any(
            linter.get("name") == d["linter"] and linter.get("status") != "success"
            for linter in report.get("linters", [])
        )
        if not fails or d["match"] not in log_of(reports, d["linter"]):
            messages.append(
                f"resolved {d['id']}: no longer reported, remove it from {DECISIONS}"
            )
    return messages, ok


def issue_body(d):
    """The decision mail: what, where, the options and the recommendation."""
    lines = [
        f"Lint report: a problem that needs your decision ({d['id']}).",
        "",
        f"- Linter: {d['linter']}",
        f"- File: `{d['file']}:{d['line']}`",
        f"- Message: {d['message']}",
        "",
        d.get("why", ""),
        "",
        "Options:",
        "",
    ]
    for i, option in enumerate(d["options"], 1):
        mark = " (recommended)" if option["name"] == d["recommended"] else ""
        lines.append(f"{i}. **{option['name']}**{mark}: {option['tradeoff']}")
    lines += [
        "",
        "Only this item waits; everything else in the lint report blocks the merge.",
        "Reply here (or in the Kompanion project) with the option you pick.",
    ]
    return "\n".join(lines)


def call(method, url, token, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(
        url,
        data,
        {"Content-Type": "application/json", "Authorization": "token " + token},
        method=method,
    )
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def open_issues(decisions, api, token):
    """One issue per waiting decision; an existing one (open or closed) is kept."""
    for d in decisions:
        if d.get("decided"):
            continue
        title = f"Lint decision: {d['id']}"
        query = urllib.parse.urlencode({"state": "all", "type": "issues", "q": title})
        found = [
            i
            for i in call("GET", f"{api}/issues?{query}", token)
            if i["title"] == title
        ]
        if found:
            print(f"{title}: issue #{found[0]['number']} exists")
            continue
        made = call(
            "POST", f"{api}/issues", token, {"title": title, "body": issue_body(d)}
        )
        print(f"{title}: opened issue #{made['number']} (mailed to the repo owner)")


def main():
    reports = os.environ.get("REPORTS", "megalinter-reports")
    path = os.path.join(reports, "mega-linter-report.json")
    if not os.path.exists(path):
        print(f"lint gate: {path} is missing (MegaLinter did not finish)")
        return 1
    report = json.load(open(path))
    decisions = json.load(open(DECISIONS)) if os.path.exists(DECISIONS) else []
    messages, ok = gate(report, decisions, reports)
    for m in messages:
        print(m)
    if os.environ.get("API") and os.environ.get("TOKEN"):
        try:
            open_issues(decisions, os.environ["API"], os.environ["TOKEN"])
        except OSError as e:
            print(f"lint gate: could not open decision issues: {e}")
    print("lint gate: " + ("passed" if ok else "FAILED: fix the problems above"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
