#!/usr/bin/env python3
"""tools/integration_gate.py (KG-09): after every N merged tasks run the full checks (build, tests, screenshot
script) and compare with a baseline. On success the tag `gate-ok` moves to HEAD. On failure it stays and a new
task is written (import file for `kompanion-server import`) with the commit range and the failing output.

Usage: integration_gate.py [--repo DIR] [--config .forgejo/integration-gate.json] [--baseline docs/integration-baseline.json]
                           [--tag gate-ok] [--force] [--task-out gate-task.json]
Exit 0: not due or passed. Exit 1: failed (task written). Run: python3 -m unittest tools/test_integration_gate.py
"""

import argparse
import datetime
import json
import os
import re
import subprocess
import sys


def git(repo, *args):
    return subprocess.run(
        ["git", *args], cwd=repo, capture_output=True, text=True, check=True
    ).stdout.strip()


def merges_since(repo, tag):
    """Merge commits on HEAD since `tag`, oldest first: {sha, pr, title}."""
    try:
        out = git(
            repo, "log", "--merges", "--reverse", "--format=%h%x09%s", f"{tag}..HEAD"
        )
    except subprocess.CalledProcessError:
        out = git(repo, "log", "--merges", "--reverse", "--format=%h%x09%s")
    merges = []
    for line in out.splitlines():
        sha, _, subject = line.partition("\t")
        m = re.match(r"Merge pull request '(.*)' \(#(\d+)\)", subject)
        merges.append(
            {
                "sha": sha,
                "pr": m.group(2) if m else "",
                "title": m.group(1) if m else subject,
            }
        )
    return merges


def due(repo, tag, every):
    return len(merges_since(repo, tag)) >= every


def tail(text, limit=1500):
    text = text.strip()
    return text if len(text) <= limit else "...\n" + text[-(limit - 4) :]


def run_check(repo, check, timeout=3600):
    try:
        p = subprocess.run(
            check["cmd"],
            shell=True,
            cwd=repo,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return {
            "name": check["name"],
            "ok": p.returncode == 0,
            "code": p.returncode,
            "output": (p.stdout + p.stderr),
        }
    except subprocess.TimeoutExpired:
        return {"name": check["name"], "ok": False, "code": -1, "output": "timed out"}


def numbers(name, output):
    """`<name>: N passed` (or `N passed`) in the output, for the baseline comparison."""
    m = re.search(rf"(?:{re.escape(name)}:\s*)?(\d+) passed", output)
    return int(m.group(1)) if m else None


def run_gate(repo, cfg, tag, baseline):
    merges = merges_since(repo, tag)
    base = (
        git(repo, "rev-parse", "--short", tag)
        if subprocess.run(
            ["git", "rev-parse", "-q", "--verify", tag], cwd=repo, capture_output=True
        ).returncode
        == 0
        else None
    )
    head = git(repo, "rev-parse", "--short", "HEAD")
    problems = []
    for check in cfg["checks"]:
        r = run_check(repo, check)
        if not r["ok"]:
            problems.append((check["name"], f"exit {r['code']}\n{tail(r['output'])}"))
            continue
        want = baseline.get(check["name"])
        got = numbers(check["name"], r["output"])
        if want is not None and got is not None and got < want:
            problems.append((check["name"], f"{got} < baseline {want}"))
    if not problems:
        subprocess.run(
            ["git", "tag", "-f", tag, "HEAD"], cwd=repo, check=True, capture_output=True
        )
        return {"ok": True, "task": None, "head": head}
    rng = f"{base}..{head}" if base else f"..{head}"
    lines = [
        f"The integration gate failed on {rng} after {len(merges)} merged task(s).",
        "",
        "Merged since the last good gate:",
    ]
    lines += [f"- #{m['pr']} {m['title']} ({m['sha']})" for m in merges] or [
        "- (none found)"
    ]
    lines += ["", "Failures:"]
    for name, why in problems:
        lines += [f"## {name}", why, ""]
    lines += [
        "Done when: the failing checks pass on main again (find the commit with `git bisect` over the range)."
    ]
    task = {
        "title": "Integration gate failed: " + ", ".join(n for n, _ in problems),
        "description": "\n".join(lines),
    }
    return {"ok": False, "task": task, "head": head}


def import_file(task, project, day):
    tid = (
        "gate-"
        + re.sub(r"[^a-z0-9]+", "-", day.lower())
        + "-"
        + re.sub(r"[^a-z0-9]+", "-", task["title"].lower())[:40].strip("-")
    )
    return {
        "projects": [
            {
                "id": "gate-" + project,
                "name": project,
                "tasks": [
                    {
                        "id": tid,
                        "title": task["title"],
                        "description": task["description"],
                        "state": "queued",
                    }
                ],
            }
        ]
    }


def load(path, default):
    if path and os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    return default


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", default=".")
    ap.add_argument("--config", default=".forgejo/integration-gate.json")
    ap.add_argument("--baseline", default="docs/integration-baseline.json")
    ap.add_argument("--tag", default="gate-ok")
    ap.add_argument("--task-out", default="gate-task.json")
    ap.add_argument("--force", action="store_true")
    a = ap.parse_args(argv)
    cfg = load(os.path.join(a.repo, a.config), {"every": 5, "checks": []})
    if not cfg["checks"]:
        print("no checks configured")
        return 0
    if not a.force and not due(a.repo, a.tag, cfg.get("every", 5)):
        print(
            f"not due: {len(merges_since(a.repo, a.tag))} of {cfg.get('every', 5)} merges"
        )
        return 0
    out = run_gate(a.repo, cfg, a.tag, load(os.path.join(a.repo, a.baseline), {}))
    if out["ok"]:
        print(f"gate passed at {out['head']}; tag {a.tag} moved")
        return 0
    day = datetime.date.today().isoformat()
    with open(a.task_out, "w", encoding="utf-8") as f:
        json.dump(
            import_file(out["task"], cfg.get("project", "kreative-kompanion"), day),
            f,
            indent=2,
        )
    print(out["task"]["title"] + "\n" + out["task"]["description"] + f"\ntask written to {a.task_out}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
