"""GATE-IN-CI: post the review gate result of every open PR as the commit status review-gate / gate."""

import re
import subprocess


def status_for(code):
    if code == 0:
        return ("success", "review findings are all handled")
    if code == 2:
        return ("pending", "PR-Agent is still reviewing the head")
    if code == 1:
        return ("failure", "review findings are open or unchecked")
    return ("error", "the review gate could not run (exit %d)" % code)


def sweep(api, gate):
    """Post the gate result on the head of every open PR that is not a draft; {number: state}."""
    results = {}
    for pull in api.open_pulls():
        if re.match(r"\s*(\[wip\]|wip\b|draft\b)", pull.get("title") or "", re.I):
            continue
        number = pull["number"]
        try:
            code = gate(number)
        except (OSError, subprocess.CalledProcessError):
            code = 3
        state, description = status_for(code)
        api.post_status(pull["head"]["sha"], state, description)
        results[number] = state
    return results
