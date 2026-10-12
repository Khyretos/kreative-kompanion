"""GATE-IN-CI: Forgejo client that posts the commit status review-gate / gate."""

import importlib.util
import json
import os
import sys
import urllib.error
import urllib.request

CONTEXT = "review-gate / gate"


class StatusClient:
    """Forgejo client for posting commit statuses."""

    def __init__(self, api: str, repo: str, token: str):
        self.base = api.rstrip("/") + "/repos/" + repo
        self.token = token

    def open_pulls(self) -> list[dict]:
        """GET /pulls?state=open&limit=50&page=N; stop on empty page."""
        out: list[dict] = []
        page = 1
        while True:
            req = urllib.request.Request(
                self.base + f"/pulls?state=open&limit=50&page={page}",
                headers={"Authorization": "token " + self.token},
            )
            with urllib.request.urlopen(req, timeout=30) as r:
                if r.status != 200:
                    raise OSError(f"GET /pulls: HTTP {r.status}")
                data = json.loads(r.read())
            if not data:
                return out
            out.extend(data)
            page += 1

    def post_status(self, sha: str, state: str, description: str) -> None:
        """POST /statuses/{sha} with {state, context: CONTEXT, description}."""
        body = json.dumps(
            {"state": state, "context": CONTEXT, "description": description}
        ).encode()
        req = urllib.request.Request(
            self.base + f"/statuses/{sha}",
            data=body,
            headers={
                "Authorization": "token " + self.token,
                "Content-Type": "application/json",
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                if r.status != 201:
                    raise OSError(f"POST status: HTTP {r.status}")
        except urllib.error.HTTPError as e:
            raise OSError(f"POST status: HTTP {e.code}")


def main(argv=None, env=None):
    env = os.environ if env is None else env
    api = env.get("FORGEJO_API")
    repo = env.get("REPO")
    token = env.get("STATUS_TOKEN")
    if not api or not repo or not token:
        print("FORGEJO_API, REPO and STATUS_TOKEN are required", file=sys.stderr)
        return 1
    spec_auto = importlib.util.spec_from_file_location(
        "auto_merge", os.path.join(os.path.dirname(__file__), "auto_merge.py")
    )
    spec_review = importlib.util.spec_from_file_location(
        "review_status", os.path.join(os.path.dirname(__file__), "review_status.py")
    )
    auto_merge = importlib.util.module_from_spec(spec_auto)
    spec_auto.loader.exec_module(auto_merge)
    review_status = importlib.util.module_from_spec(spec_review)
    spec_review.loader.exec_module(review_status)
    client = StatusClient(api, repo, token)
    read_token = env.get("READ_TOKEN") or token
    gate = auto_merge.make_gate(read_token, api, repo)
    results = review_status.sweep(client, gate)
    for n in sorted(results):
        print(f"PR #{n}: {results[n]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
