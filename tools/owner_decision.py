#!/usr/bin/env python3
"""GATE-02: Apply lint decisions from repo owner comments.

Runs in a Forgejo Actions job when someone comments on an issue or pull request.
Only the repo owner's comments count: the owner answers a decision by comment and
this script applies it, records it and closes the item, without Claude.

Usage (Forgejo Actions, repo root): python3 tools/owner_decision.py

Env vars:
  GITHUB_EVENT_PATH (the issue_comment event JSON)
  API (repo API URL)
  TOKEN (job token: comments, closing, pushing)
  PR_TOKEN (opens pull requests so CI runs; falls back to TOKEN)
  MODEL_BASE + OVMS_API_KEY (Coder for option jobs)
"""

import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from datetime import date, timedelta

DECISIONS = ".forgejo/lint-decisions.json"
PREFIX = "Lint decision: "


def call(method: str, url: str, token: str, body=None) -> dict:
    """JSON request with 'Authorization: token ...' ({} for an empty answer); errors raise."""
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(
        url,
        data,
        {"Content-Type": "application/json", "Authorization": "token " + token},
        method=method,
    )
    with urllib.request.urlopen(req, timeout=30) as r:
        text = r.read()
    return json.loads(text) if text.strip() else {}


def git(*args) -> str:
    """Run git command and return stdout."""
    result = subprocess.run(["git", *args], check=True, capture_output=True, text=True)
    return result.stdout


def owner_comment(api: str, token: str, comment_id: int) -> str | None:
    """GET comment and GET repo owner login. Return body if user matches owner, else None."""
    c = call("GET", f"{api}/issues/comments/{comment_id}", token)
    if not c:
        return None
    r = call("GET", api, token)
    if not r:
        return None
    owner_login = r.get("owner", {}).get("login")
    user_login = c.get("user", {}).get("login")
    if user_login != owner_login:
        return None
    return c.get("body")


def parse_choice(body: str, options: list, recommended: str) -> int | None:
    """Parse option index or None from comment body."""
    text = "\n".join(l for l in body.lower().splitlines() if not l.lstrip().startswith(">") and not l.rstrip().endswith("?"))
    m = re.search(r"\b(?:option|accept|pick|choose|go with)\s+(?:option\s+)?#?(\d+)\b", text)
    if m:
        n = int(m.group(1))
        return n - 1 if 1 <= n <= len(options) else None
    for i, o in enumerate(options):
        if o["name"].lower() in text:
            return i
    if re.search(r"\baccept\b", text):
        for i, o in enumerate(options):
            if o["name"] == recommended:
                return i
    return None


def apply_lint(decision: dict, index: int, who: str, today: str, url: str, root: str) -> list:
    """Apply decision, update .trivyignore if needed, return pipeline jobs."""
    option = decision["options"][index]
    decision["decided"] = f"{option['name']} ({who}, {today}, {url})"
    if option.get("keep_waiting"):
        decision["keep_waiting"] = True
    
    jobs = list(option.get("jobs", []))
    
    if "apply" in option and "trivyignore" in option["apply"]:
        ignore_id = option["apply"]["trivyignore"]
        days = option["apply"].get("days", 92)
        expiry = date.fromisoformat(today) + timedelta(days=days)
        
        trivypath = os.path.join(root, ".trivyignore")
        lines = []
        if os.path.exists(trivypath):
            with open(trivypath) as f:
                lines = f.read().splitlines()
            f.close()
        if not any(line.startswith(ignore_id + " ") for line in lines):
            lines += [
                f"# {decision['id']}: {decision['decided']}",
                f"{ignore_id} exp:{expiry.isoformat()}",
            ]
        with open(trivypath, "w") as f:
            f.write("\n".join(lines) + "\n")
            f.close()
    
    return jobs


def parse_verdicts(body: str, ledger: dict) -> dict:
    """Parse verdicts from comment body into {id: verdict}."""
    verdicts = {}
    named = False
    lines = [l for l in body.lower().splitlines() if not l.lstrip().startswith(">")]
    
    for line in lines:
        if line.lstrip().startswith(">"):
            continue
        for m in re.finditer(r"[a-f0-9]{6,12}", line):
            hits = [fid for fid in ledger if fid.startswith(m.group())]
            if len(hits) != 1:
                continue
            fid = hits[0]
            named = True
            after = line[m.end():]
            before = line[:m.start()]
            if re.match(r"\s*:?\s*(false[ _]positive|fp)", after):
                verdicts[fid] = "false_positive"
            elif re.match(r"\s*:?\s*bug", after):
                verdicts[fid] = "bug"
            elif "accept" in before and ledger[fid].get("verdict") in ("bug", "false_positive"):
                verdicts[fid] = ledger[fid]["verdict"]
    
    if not named and re.search(r"\baccept\b", "\n".join(l for l in body.lower().splitlines() if not l.lstrip().startswith(">"))):
        for fid, e in ledger.items():
            if not e.get("checked") and e.get("verdict") in ("bug", "false_positive"):
                verdicts[fid] = e["verdict"]
    
    return verdicts


def record_verdicts(ledger: dict, verdicts: dict, who: str) -> list:
    """Record verdicts, mark checked, return sorted bug ids with no fix."""
    for fid, v in verdicts.items():
        ledger[fid]["verdict"] = v
        ledger[fid]["checked"] = True
        ledger[fid]["by"] = who
    
    bugs = []
    for fid, entry in ledger.items():
        if entry.get("checked") and entry.get("verdict") == "bug" and not entry.get("fixed"):
            bugs.append(fid)
    return sorted(bugs)


def check_for(path: str) -> str:
    """Return shell check command for file type."""
    ext = os.path.splitext(path)[1].lower()
    if ext == ".py":
        return f"python3 -m py_compile {path}"
    if ext == ".rs":
        return "cd server && cargo check -q"
    if ext in (".js", ".mjs"):
        return f"node --check {path}"
    return "true"


def run_pipeline(jobs: list, log: str) -> None:
    """Write temp kompanion.toml and jobs JSON, then run pipeline.py."""
    toml_content = '''[[provider]]
id = "ovms"
kind = "openai-compatible"
base_url = "$MODEL_BASE"
api_key_env = "OVMS_API_KEY"
extra_body = { chat_template_kwargs = { enable_thinking = false } }

[roles]
worker = { provider = "ovms", model = "Coder" }
'''.replace("$MODEL_BASE", os.environ["MODEL_BASE"])
    with tempfile.NamedTemporaryFile(mode="w", suffix=".toml", delete=False) as f:
        f.write(toml_content)
        toml_path = f.name
    
    tmpdir = tempfile.mkdtemp()
    jobs_path = os.path.join(tmpdir, "jobs.json")
    with open(jobs_path, "w") as f:
        json.dump(jobs, f, indent=2)
    
    env = dict(os.environ, KOMPANION_CONFIG=toml_path)
    subprocess.run([sys.executable, "tools/qwen/pipeline.py", jobs_path, log], env=env, check=True)
    
    os.unlink(toml_path)
    shutil.rmtree(tmpdir)


def comment(api: str, token: str, number: int, text: str) -> dict:
    """POST comment to issue."""
    return call("POST", f"{api}/issues/{number}/comments", token, {"body": text})


def lint_flow(api: str, token: str, pr_token: str, issue: dict, body: str, who: str, url: str, today: str) -> int:
    """Handle decision issue flow."""
    with open(DECISIONS) as f:
        decisions = json.load(f)
    title = issue["title"]
    if not title.startswith(PREFIX):
        print(f"Ignored: issue title does not start with '{PREFIX}'")
        return 0
    
    d_id = title[len(PREFIX):]
    d = next((item for item in decisions if item["id"] == d_id), None)
    if not d:
        comment(api, token, issue["number"], f"No decision with this id in {DECISIONS}.")
        return 0
    
    if d.get("decided") and not d.get("keep_waiting"):
        comment(api, token, issue["number"], f"Already decided: {d['decided']}")
        return 0
    
    index = parse_choice(body, d["options"], d["recommended"])
    if index is None:
        print(f"Ignored: could not parse choice from comment")
        return 0
    
    jobs = apply_lint(d, index, who, today, url, ".")
    
    with open(DECISIONS, "w") as f:
        json.dump(decisions, f, indent=2)
        f.write("\n")
    
    branch = f"decision/{d['id']}"
    git("checkout", "-q", "-B", branch)
    
    if jobs:
        run_pipeline(jobs, f"docs/qwen-log/{branch.replace('/', '-')}.jsonl")
    
    git("add", "-A")
    name = d["options"][index]["name"]
    git("commit", "-q", "-m", f"GATE-02: lint decision {d['id']}: {name} (picked by {who})")
    git("push", "-q", "-f", "origin", f"HEAD:refs/heads/{branch}")
    
    pr = call("POST", f"{api}/pulls", pr_token, {
        "head": branch,
        "base": "main",
        "title": f"[GATE-02] Lint decision {d['id']}: {name}",
        "body": f"Picked option {index + 1}: **{name}**.\n\nPicked by @{who} at {url}.\n\nDecision issue: #{issue['number']}."
    })
    
    note = f"Picked option {index + 1}: **{name}** (by @{who}). Pull request #{pr['number']} applies it and records it in {DECISIONS}; merge it once CI is green."
    comment(api, token, issue["number"], note)
    
    call("PATCH", f"{api}/issues/{issue['number']}", token, {"state": "closed"})
    return 0


def findings_flow(api: str, token: str, pr_token: str, issue: dict, body: str, who: str, today: str) -> int:
    """Handle PR findings flow."""
    n = issue["number"]
    spec = importlib.util.spec_from_file_location("review_findings", os.path.join("tools", "qwen", "review_findings.py"))
    rf = importlib.util.module_from_spec(spec); spec.loader.exec_module(rf)
    state = None
    for c in reversed(call("GET", f"{api}/issues/{n}/comments?limit=50", token)):
        state = rf.state_from_body(c.get("body", ""))
        if state: break
    if not state: print("ignored: no PR-Agent review on this pull request"); return 0
    findings = rf.open_findings(state)
    git("fetch", "-q", "origin", f"refs/pull/{n}/head")
    git("checkout", "-q", "-B", f"findings/pr-{n}", "FETCH_HEAD")
    path = os.path.join("docs", "review-findings", f"pr-{n}.json")
    if os.path.exists(path):
        with open(path) as f:
            ledger = json.load(f)
    else:
        ledger = {}
    for f in findings:
        ledger.setdefault(f["finding_id"], {"title": rf.title(f), "path": f["path"], "verdict": "", "checked": False, "fixed": ""})
    verdicts = parse_verdicts(body, ledger)
    if not verdicts: print("ignored: no verdict in the comment"); return 0
    bugs = record_verdicts(ledger, verdicts, who)
    lines = [f"Recorded (by @{who}):"] + [f"- `{fid}` {ledger[fid]['title']}: {v}" for fid, v in sorted(verdicts.items())]
    if bugs:
        jobs = rf.fix_jobs(findings, ledger)
        for job in jobs: job["check"] = check_for(job["out"])
        run_pipeline(jobs, os.path.join("docs", "qwen-log", f"findings-pr-{n}.jsonl"))
        git("add", "-A"); git("commit", "-q", "-m", f"GATE-02: Coder's fixes for review findings of #{n}")
        sha = git("rev-parse", "HEAD").strip()
        for fid in bugs: ledger[fid]["fixed"] = sha
        with open(path, "w") as f: json.dump(ledger, f, indent=2, sort_keys=True); f.write("\n")
        git("add", "-A"); git("commit", "-q", "-m", f"GATE-02: owner's verdicts for #{n}")
        git("push", "-q", "-f", "origin", f"HEAD:refs/heads/findings/pr-{n}")
        pr = call("POST", f"{api}/pulls", pr_token, {"head": f"findings/pr-{n}", "base": "main", "title": f"[GATE-02] Review fixes for #{n}", "body": f"Carries #{n}'s commits plus Coder's fixes for the review findings @{who} confirmed as bugs, and the updated ledger {path}. Merging it also merges #{n}."})
        lines.append(f"Coder's fixes for the bugs: pull request #{pr['number']} (merge it instead of #{n}).")
    else:
        lines.append("No code change needed; the review gate reads these verdicts from this comment.")
    still = [(fid, e) for fid, e in sorted(ledger.items()) if not e.get("checked")]
    if still:
        lines.append("Still open:")
        lines += [f"- `{fid}` {e['title']} ({e['path']}), proposed: {e.get('verdict') or 'none'}; answer '{fid} bug' or '{fid} false positive'" for fid, e in still]
    comment(api, token, n, "\n".join(lines))
    return 0


def main() -> int:
    with open(os.environ["GITHUB_EVENT_PATH"]) as f:
        event = json.load(f)
    api = os.environ["API"]
    token = os.environ["TOKEN"]
    pr_token = os.environ.get("PR_TOKEN") or token
    
    body = owner_comment(api, token, event["comment"]["id"])
    if body is None:
        print("ignored: the comment is not from the repo owner")
        return 0
    
    who = event["repository"]["owner"]["login"]
    url = event["comment"]["html_url"]
    today = date.today().isoformat()
    issue = event["issue"]
    
    if issue["title"].startswith(PREFIX):
        return lint_flow(api, token, pr_token, issue, body, who, url, today)
    
    if issue.get("pull_request"):
        return findings_flow(api, token, pr_token, issue, body, who, today)
    
    print("ignored: not a decision item")
    return 0


if __name__ == "__main__":
    sys.exit(main())
