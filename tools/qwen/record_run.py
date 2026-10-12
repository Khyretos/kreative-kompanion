"""Run pipeline.py's main() with every model call recorded (prompt + answer) in $TRACE_OUT. Usage: trace-driver.py TREE jobs.json log.jsonl"""
import importlib
import json
import os
import sys
import time

tree, jobs, log = sys.argv[1:4]
sys.path.insert(0, os.path.join(tree, "tools", "qwen"))
pipeline = importlib.import_module("pipeline")
out = os.environ["TRACE_OUT"]
os.makedirs(out, exist_ok=True)
orig = pipeline.ask


def ask(system, user, max_tokens):
    a, s, t = orig(system, user, max_tokens)
    with open(
        os.path.join(out, f"{int(time.time()*1000)}.json"), "w", encoding="utf-8"
    ) as f:
        json.dump(
            {
                "system": system,
                "user": user,
                "answer": a,
                "completion_tokens": t,
                "seconds": s,
            },
            f,
            ensure_ascii=False,
            indent=1,
        )
    return a, s, t


pipeline.ask = ask
sys.argv = ["pipeline.py", jobs, log]
pipeline.main()
