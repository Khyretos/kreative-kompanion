"""RVW-01: act on PR-Agent's review findings before a merge (drafted by Coder, fixed by Claude).

PR-Agent keeps its open "Recommended focus areas" as JSON in a hidden block of its
"PR Reviewer Guide" comment. This compares them to the ledger
docs/review-findings/pr-<n>.json and gates the merge.

  review_findings.py <pr> --triage       Coder gives each new finding a verdict (bug / false_positive)
  (Claude reads each verdict, corrects it, sets "checked": true)
  review_findings.py <pr> --jobs j.json  pipeline patch jobs for the checked bugs (Claude adds the checks)
  (after the fix commit Claude sets "fixed": "<sha>" and pushes; PR-Agent reviews the push)
  review_findings.py <pr>                the gate: exit 0 all handled, 1 blocked, 2 review pending
"""

import argparse
import json
import os
import re
import sys
import urllib.request

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Import pipeline only inside the triage function to avoid config needs for tests.
# pylint: disable=import-outside-toplevel


def state_from_body(body: str):
    """Parse the hidden JSON block from a PR-Agent comment body.
    
    Returns the dict if the marker "<!-- pr-agent-review-state:v1\n" + JSON + "\n-->"
    is found, otherwise None.
    """
    pattern = r'<!-- pr-agent-review-state:v1\n(.*?)\n-->'
    match = re.search(pattern, body, re.S)
    if not match:
        return None
    try:
        return json.loads(match.group(1))
    except json.JSONDecodeError:
        return None


def open_findings(state: dict):
    """Return the list of findings with state 'ACTIVE', in order."""
    return [f for f in state.get("findings", []) if f.get("state") == "ACTIVE"]


def title(finding: dict) -> str:
    """Extract the title from a finding's body.
    
    The first line should be "**Title**". Falls back to the stripped first line.
    """
    body = finding.get("body", "")
    lines = body.split("\n")
    if not lines:
        return ""
    first_line = lines[0]
    # Try to extract text between ** and **
    match = re.match(r'\*\*(.+?)\*\*', first_line)
    if match:
        return match.group(1)
    return first_line.strip()


def blocking(findings: list, ledger: dict) -> list:
    """Return a list of (finding_id, why) for each non-cleared finding.
    
    A finding is cleared if:
      - verdict is "false_positive" and checked is True, OR
      - verdict is "bug", checked is True and fixed is set (non-empty).
    """
    results = []
    for f in findings:
        fid = f["finding_id"]
        entry = ledger.get(fid)
        if entry is None:
            results.append((fid, "not triaged"))
            continue
        
        checked = entry.get("checked", False)
        verdict = entry.get("verdict")
        fixed = entry.get("fixed", "")
        
        if verdict == "false_positive" and checked:
            continue
        if entry.get("checked") is not True:
            results.append((fid, "verdict not checked by Claude"))
        elif verdict == "bug" and not fixed:
            results.append((fid, "bug not fixed yet"))
    
    return results


def parse_verdict(text: str) -> dict:
    """Parse the first {...} JSON object from text into {"verdict", "reason"}.
    
    Raises ValueError if no JSON object is found or verdict is invalid.
    """
    match = re.search(r"\{.*?\}", text, re.S)
    if not match:
        raise ValueError("No JSON object found")
    try:
        obj = json.loads(match.group())
    except json.JSONDecodeError:
        raise ValueError("Invalid JSON object")
    
    if "verdict" not in obj:
        raise ValueError("Missing verdict key")
    
    verdict = obj["verdict"]
    if verdict not in ("bug", "false_positive"):
        raise ValueError(f"Invalid verdict: {verdict}")
    
    reason = obj.get("reason", "")
    return {"verdict": verdict, "reason": reason}


def excerpt(path: str, start: int, end: int, radius: int = 40, cap: int = 400) -> str:
    """Return a numbered excerpt of a file.
    
    Lines are formatted as "<n>: <line>" (1-based).
    If start/end are None, returns the first `cap` lines.
    If the file does not exist, returns exactly "(file not in this checkout)".
    """
    full_path = path
    if not os.path.isfile(full_path):
        return "(file not in this checkout)"
    
    try:
        with open(full_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except Exception:
        return "(file not in this checkout)"
    
    if start is None and end is None:
        limit = min(cap, len(lines))
        return "".join(f"{i+1}: {lines[i]}" for i in range(limit))
    
    start_idx = max(0, start - radius - 1)
    end_idx = min(len(lines), end + radius)
    
    result_lines = []
    for i in range(start_idx, end_idx):
        line_content = lines[i]
        # Strip trailing newline but keep internal newlines if any
        line_content = line_content.rstrip("\n\r")
        result_lines.append(f"{i+1}: {line_content}")
    
    return "\n".join(result_lines) + "\n"


def locate(path: str, body: str):
    """The first line (1-based) of the file that holds a `name` the finding quotes, else None.
    Findings without line numbers would otherwise show only the top of a long file."""
    # Longest first: `overseer-interject` says more than `answer`.
    names = sorted(set(re.findall(r"[`\"']([A-Za-z_][A-Za-z0-9_-]{2,})[`\"']", body)), key=len, reverse=True)
    try:
        lines = open(path, encoding="utf-8").read().splitlines()
    except OSError:
        return None
    for name in names:
        for i, line in enumerate(lines):
            if re.search(r"\b" + re.escape(name) + r"\b", line):
                return i + 1
    return None


def role_for(path: str) -> str:
    """Determine the worker role for a file path."""
    ext = os.path.splitext(path)[1].lower()
    if ext == ".rs":
        return "worker/rust"
    if ext in (".ts", ".tsx", ".js", ".css", ".html"):
        return "worker/web"
    return "worker"


def fix_jobs(findings: list, ledger: dict) -> list:
    """Generate pipeline jobs for unchecked, unfixed bugs.
    
    Only includes findings where:
      - verdict is "bug"
      - checked is True
      - fixed is empty/missing
    """
    jobs = []
    for f in findings:
        fid = f["finding_id"]
        entry = ledger.get(fid)
        if not entry:
            continue
        if entry.get("verdict") != "bug":
            continue
        if not entry.get("checked", False):
            continue
        if entry.get("fixed"):
            continue
        
        path = f["path"]
        prompt_parts = [
            f"Fix issue: {title(f)}",
            f"Body: {f['body']}",
            f"Claude's reason: {entry.get('reason', '')}",
            f"Lines:\n{excerpt(os.path.join(REPO, path), f.get('line_start'), f.get('line_end'))}",
            "Fix only this problem, change nothing else."
        ]
        jobs.append({
            "name": f"fix-{fid}",
            "role": role_for(path),
            "mode": "patch",
            "prompt": "\n".join(prompt_parts),
            "context": [path],
            "out": path,
            "check": ""
        })
    return jobs


def triage(pr: dict, findings: list, ledger: dict, repo_root: str):
    """Triage new findings by asking the model for a verdict."""
    # Import pipeline here to avoid needing config for unit tests.
    import pipeline
    
    new_findings = [f for f in findings if f["finding_id"] not in ledger]
    
    for f in new_findings:
        fid = f["finding_id"]
        path = f["path"]
        line_start = f.get("line_start")
        line_end = f.get("line_end")
        
        full = os.path.join(repo_root, path)
        if line_start is None:
            line_start = line_end = locate(full, f.get("body", ""))
        excerpt_text = excerpt(full, line_start, line_end)
        
        system = "You triage code review findings. Answer only with JSON: {\"verdict\": \"bug\" or \"false_positive\", \"reason\": \"one sentence\"}. A bug is a real defect in the code shown that a test could show. Vague advice, style, missing logging, things already handled in the code shown, and claims about code not shown are false_positive."
        user = f"Finding:\n{f['body']}\n\nPath: {path}\nCode:\n{excerpt_text}"
        
        verdict_entry = None
        for attempt in range(2):
            try:
                text, secs, tokens = pipeline.ask(system, user, max_tokens=512)
                verdict_entry = parse_verdict(text)
                break
            except Exception:
                if attempt == 0:
                    continue
                # Retry failed; record default verdict
                verdict_entry = {"verdict": "bug", "reason": "unparsed model answer, Claude decides"}
        
        entry = {
            "title": title(f),
            "path": path,
            "verdict": verdict_entry["verdict"],
            "reason": verdict_entry["reason"],
            "by": "coder",
            "checked": False,
            "fixed": ""
        }
        ledger[fid] = entry
        
        print(f"{fid} {entry['verdict']}: {entry['title']} ({entry['path']}) - {entry['reason']}")


def main():
    parser = argparse.ArgumentParser(description="Act on PR-Agent review findings.")
    parser.add_argument("pr", type=int, help="PR number")
    parser.add_argument("--repo", default="khyretos/kreative-kompanion", help="Owner/repo")
    parser.add_argument("--triage", action="store_true", help="Run triage and update ledger")
    parser.add_argument("--jobs", help="Write fix jobs as JSON to this path")
    parser.add_argument("--allow-stale", action="store_true",
                        help="Use the last review even when it is for an older commit (a push review that failed)")
    args = parser.parse_args()
    
    base_url = "https://git.kreative-kompas.com/api/v1"
    env_file = os.environ.get("PR_AGENT_ENV", "/home/khyretos/Docker/Services/pr-agent/.env")
    
    # Read token
    token = None
    if os.path.exists(env_file):
        with open(env_file, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line.startswith("GITEA__PERSONAL_ACCESS_TOKEN="):
                    token = line.split("=", 1)[1].strip('"\'')
                    break
    
    if not token:
        print("Error: GITEA__PERSONAL_ACCESS_TOKEN not found", file=sys.stderr)
        sys.exit(1)
    
    headers = {"Authorization": f"token {token}"}
    
    owner, repo = args.repo.split("/")
    pr_num = args.pr
    
    head_sha = get_json(f"{base_url}/repos/{owner}/{repo}/pulls/{pr_num}", headers)["head"]["sha"]
    comments_data = get_json(f"{base_url}/repos/{owner}/{repo}/issues/{pr_num}/comments?limit=50", headers)

    # The newest PR-Agent review comment
    state = None
    for c in reversed(comments_data):
        if "<!-- pr-agent-review-state:v1" in c.get("body", ""):
            state = state_from_body(c["body"])
            if state:
                break
    
    if not state:
        print("review pending: no PR-Agent review yet")
        sys.exit(2)
    
    last_run = state.get("last_run", {})
    if last_run.get("head_sha") != head_sha and not args.allow_stale:
        old_sha = last_run.get("head_sha", "")[:7]
        new_sha = head_sha[:7]
        print(f"review pending: last review is for {old_sha}, head is {new_sha}")
        sys.exit(2)
    
    # Load ledger
    repo_root = REPO
    ledger_path = os.path.join(repo_root, "docs", "review-findings", f"pr-{pr_num}.json")
    ledger = {}
    if os.path.exists(ledger_path):
        with open(ledger_path, "r", encoding="utf-8") as f:
            ledger = json.load(f)
    
    findings = open_findings(state)
    
    if args.triage:
        triage({"number": pr_num}, findings, ledger, repo_root)
        os.makedirs(os.path.dirname(ledger_path), exist_ok=True)
        with open(ledger_path, "w", encoding="utf-8") as f:
            json.dump(ledger, f, indent=2, sort_keys=True)
    
    if args.jobs:
        jobs = fix_jobs(findings, ledger)
        with open(args.jobs, "w", encoding="utf-8") as f:
            json.dump(jobs, f, indent=2)
    
    # Gate check
    blocking_list = blocking(findings, ledger)
    if blocking_list:
        by_id = {f["finding_id"]: f for f in findings}
        for fid, why in blocking_list:
            print(f"BLOCK {fid} {why}: {title(by_id[fid])} ({by_id[fid].get('path', '?')})")
        sys.exit(1)
    
    print(f"review findings: all {len(findings)} open findings handled")
    sys.exit(0)


def get_json(url: str, headers: dict):
    """GET a Forgejo API URL and decode the JSON answer."""
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.load(resp)


if __name__ == "__main__":
    main()
