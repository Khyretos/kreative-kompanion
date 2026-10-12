"""tools/auto_merge.py (CI-MERGE). Run: python3 -m unittest tools/test_auto_merge.py"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from typing import Any, Callable, Dict, List, Optional, Tuple

# Default configuration
DEFAULTS = {
    "required": [
        "build / skills",
        "build / runner",
        "build / web",
        "build / server",
        "build / web-live",
        "lint / megalinter",
    ],
    "opt_out_label": "no-auto-merge",
    "own_context": "auto-merge /",
    "base": "main",
}


def load_config(path: str) -> Dict[str, Any]:
    """Load config from JSON file, merging into DEFAULTS."""
    cfg = DEFAULTS.copy()
    if os.path.exists(path):
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
            cfg.update(data)
        except (json.JSONDecodeError, IOError):
            pass
    return cfg


def skip_reason(pr: Dict[str, Any], cfg: Dict[str, Any]) -> Optional[str]:
    """Determine if a PR should be skipped and why."""
    # Closed or merged
    if pr.get("state") == "closed" or pr.get("merged"):
        return "PR closed or already merged"

    # Draft flag
    if pr.get("draft"):
        return "PR is a draft"

    if re.match(r"\s*(\[wip\]|wip\b|draft\b)", pr.get("title", ""), re.I):
        return "PR title marks a draft"

    # Opt-out label
    labels = pr.get("labels", [])
    for label in labels:
        if isinstance(label, dict) and label.get("name") == cfg["opt_out_label"]:
            return "PR has opt-out label"

    # Fork check
    head_repo = pr.get("head", {}).get("repo", {})
    base_repo = pr.get("base", {}).get("repo", {})
    if head_repo.get("full_name") != base_repo.get("full_name"):
        return "PR is from a fork"

    # Base ref check
    base_ref = pr.get("base", {}).get("ref", "")
    if base_ref != cfg["base"]:
        return f"Base branch '{base_ref}' does not match expected '{cfg['base']}'"

    return None


def check_state(statuses: List[Dict[str, str]], cfg: Dict[str, Any]) -> Tuple[str, str]:
    """Check commit statuses and return state and reason."""
    required = set(cfg["required"])

    # Filter out own_context and strip trailing ' (event)'
    filtered_statuses: Dict[str, str] = {}
    for s in statuses:
        ctx = s.get("context", "")
        ctx = re.sub(r" \([a-z_]+\)$", "", ctx)
        # Ignore own_context
        if ctx.startswith(cfg["own_context"]):
            continue

        # Last entry wins
        filtered_statuses[ctx] = s.get("status", "")

    # Check for failures/errors first (any status, required or not)
    for ctx, status in filtered_statuses.items():
        if status in ("failure", "error"):
            return ("blocked", f"Status {ctx} failed with {status}")

    # Check required checks
    missing_or_pending = []
    for req in required:
        if req not in filtered_statuses:
            missing_or_pending.append(req)
        elif filtered_statuses[req] == "pending":
            missing_or_pending.append(req)

    if missing_or_pending:
        return ("wait", f"Missing or pending: {', '.join(missing_or_pending)}")

    # No statuses at all
    if not filtered_statuses:
        return ("wait", "No statuses found")

    # All required checks are present and passing
    return ("ok", "All checks passed")

    # If we get here, all checks passed but we still need to check mergeability
    # This path is unreachable because check_state returns early on success


def process(
    n: int,
    api: Any,
    cfg: Dict[str, Any],
    gate: Callable[[int], int],
    dry_run: bool = False,
) -> Tuple[str, str]:
    """Process a single pull request."""
    # Get initial PR data and SHA
    pr = api.pull(n)
    initial_sha = pr["head"]["sha"]

    # Skip check
    reason = skip_reason(pr, cfg)
    if reason:
        return ("skip", reason)

    # State check
    state, reason = check_state(api.statuses(pr["head"]["sha"]), cfg)
    if state in ("wait", "blocked"):
        return (state, reason)

    # Gate check
    exit_code = gate(n)
    if exit_code == 0:
        pass
    elif exit_code == 2:
        return ("wait", "review pending")
    else:
        return ("blocked", f"Gate returned non-zero exit code: {exit_code}")

    # Re-read PR to check head SHA
    pr = api.pull(n)
    if pr["head"]["sha"] != initial_sha:
        return ("wait", "head moved")

    # Mergeable check
    if not pr.get("mergeable", True):
        return ("blocked", "conflicts")

    # Dry run
    if dry_run:
        return ("would merge", "dry run")

    # Attempt merge
    status_code = api.merge(n, initial_sha)
    if status_code in (200, 204):
        return ("merged", "merged successfully")
    elif status_code == 409:
        return ("wait", "head changed while merging")
    else:
        return ("blocked", f"merge failed with status {status_code}")


def sweep(
    api: Any,
    cfg: Dict[str, Any],
    gate: Callable[[int], int],
    dry_run: bool = False,
) -> Dict[int, Tuple[str, str]]:
    """Process all open pull requests."""
    results = {}
    for pr in api.open_pulls():
        number = pr["number"]
        outcome, reason = process(number, api, cfg, gate, dry_run)
        results[number] = (outcome, reason)
    return results


class Forgejo:
    """The few Forgejo API calls the merge needs."""

    def __init__(self, api: str, repo: str, token: str):
        self.base = f"{api.rstrip('/')}/repos/{repo}"
        self.token = token

    def _call(self, method: str, path: str, body: Any = None) -> Tuple[int, Any]:
        """(HTTP status, decoded JSON or None); an HTTP error status is returned, not raised."""
        req = urllib.request.Request(
            self.base + path,
            json.dumps(body).encode() if body is not None else None,
            {
                "Authorization": "token " + self.token,
                "Content-Type": "application/json",
            },
            method=method,
        )
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                raw = r.read()
                return r.status, json.loads(raw) if raw else None
        except urllib.error.HTTPError as e:
            return e.code, None

    def _get(self, path: str) -> Any:
        status, data = self._call("GET", path)
        if status != 200:
            raise OSError(f"GET {path}: HTTP {status}")
        return data

    def pull(self, n: int) -> Dict[str, Any]:
        return self._get(f"/pulls/{n}")

    def statuses(self, sha: str) -> List[Dict[str, str]]:
        return list(reversed(self._get(f"/commits/{sha}/statuses?limit=50")))

    def open_pulls(self) -> List[Dict[str, Any]]:
        out: List[Dict[str, Any]] = []
        page = 1
        while True:
            chunk = self._get(f"/pulls?state=open&limit=50&page={page}")
            if not chunk:
                return out
            out += chunk
            page += 1

    def merge(self, n: int, sha: str) -> int:
        body = {"Do": "merge", "head_commit_id": sha, "delete_branch_after_merge": True}
        return self._call("POST", f"/pulls/{n}/merge", body)[0]


ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def make_gate(read_token: str, api: str, repo: str) -> Callable[[int], int]:
    """gate(n): main's review_findings.py run against a checkout of the PR head (its ledger)."""

    def gate(n: int) -> int:
        git = ["git", "-C", ROOT]
        subprocess.run(
            git + ["fetch", "-q", "origin", f"+refs/pull/{n}/head:refs/pr/{n}"],
            check=True,
        )
        tree = tempfile.mkdtemp(prefix=f"pr{n}-")
        try:
            subprocess.run(
                git + ["worktree", "add", "-q", "--detach", tree, f"refs/pr/{n}"],
                check=True,
            )
            env = {k: v for k, v in os.environ.items() if k != "MERGE_TOKEN"}
            env.update(FORGEJO_API=api, FORGEJO_TOKEN=read_token, REVIEW_REPO=tree)
            cmd = [
                sys.executable,
                os.path.join(ROOT, "tools", "qwen", "review_findings.py"),
                str(n),
                "--repo",
                repo,
            ]
            return subprocess.run(cmd, env=env, check=False).returncode
        finally:
            subprocess.run(git + ["worktree", "remove", "--force", tree], check=False)
            shutil.rmtree(tree, ignore_errors=True)

    return gate


def main(argv: Optional[List[str]] = None, env: Optional[Dict[str, str]] = None) -> int:
    """CLI entry point; 0 unless misconfigured or the API failed (waiting and blocked PRs are fine)."""
    env = os.environ if env is None else env
    parser = argparse.ArgumentParser(description="Merge pull requests that are ready.")
    parser.add_argument("--pr", type=int, help="one PR (default: every open PR)")
    parser.add_argument("--dry-run", action="store_true", help="decide, do not merge")
    parser.add_argument(
        "--config", default=os.path.join(ROOT, ".forgejo", "auto-merge.json")
    )
    args = parser.parse_args(argv)

    api, repo = env.get("FORGEJO_API"), env.get("REPO")
    if not api or not repo:
        print("FORGEJO_API and REPO are required", file=sys.stderr)
        return 1
    merge_token = env.get("MERGE_TOKEN", "")
    if not merge_token and not args.dry_run:
        print("MERGE_TOKEN is required (or use --dry-run)", file=sys.stderr)
        return 1
    read_token = env.get("READ_TOKEN") or merge_token
    cfg = load_config(args.config)
    forgejo = Forgejo(api, repo, merge_token or read_token)
    reader = Forgejo(api, repo, read_token)
    forgejo.pull, forgejo.statuses, forgejo.open_pulls = (
        reader.pull,
        reader.statuses,
        reader.open_pulls,
    )
    gate = make_gate(read_token, api, repo)
    try:
        if args.pr:
            results = {args.pr: process(args.pr, forgejo, cfg, gate, args.dry_run)}
        else:
            results = sweep(forgejo, cfg, gate, args.dry_run)
    except (OSError, subprocess.CalledProcessError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    for n, (outcome, reason) in sorted(results.items()):
        print(f"PR #{n}: {outcome} - {reason}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
