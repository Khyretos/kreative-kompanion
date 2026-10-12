#!/usr/bin/env python3
"""STU-N2 check: run a vn-cast-<N> workflow on ComfyUI for several seeds and save the pictures.

Applies the workflow.toml params the way server/src/studio/workflow.rs does (template, also/add, float/int/seed).
Usage: cast_check.py N SEEDS OUTDIR [--comfy http://127.0.0.1:8188] [--cast "p1|p2|p3"] [--rating general]
Only run it when ComfyUI's /queue and Kompanion's GPU job list are empty (HANDOFF.md, GPU rule).
"""

import argparse
import copy
import json
import os
import sys
import time
import tomllib
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def build(n, seed, cast, rating="general"):
    d = os.path.join(ROOT, "studio", "workflows", f"vn-cast-{n}")
    wf = tomllib.load(open(os.path.join(d, "workflow.toml"), "rb"))
    g = copy.deepcopy(json.load(open(os.path.join(d, "graph.json"))))
    neg_word = {"general": "nsfw, nude, explicit, "}.get(rating, "")
    for p in wf["param"]:
        v = p["default"]
        if p["name"].startswith("person") and p["name"][6:].isdigit():
            i = int(p["name"][6:]) - 1
            if cast and i < len(cast):
                v = cast[i]
        if p["type"] == "seed":
            v = seed
        if p["type"] == "string":
            t = p.get("template") or "{value}"
            v = t.replace("{rating_neg}", neg_word).replace("{rating}", rating).replace("{value}", v)
        if "node" not in p:
            continue
        g[p["node"]]["inputs"][p["input"]] = v
        for a in p.get("also", []):
            g[a["node"]]["inputs"][a["input"]] = v + a.get("add", 0)
    return g


def post(url, data=None):
    req = urllib.request.Request(url, data=json.dumps(data).encode() if data else None, headers={"Content-Type": "application/json"})
    return json.load(urllib.request.urlopen(req, timeout=60))


def run(comfy, graph, out):
    pid = post(comfy + "/prompt", {"prompt": graph})["prompt_id"]
    while True:
        h = json.load(urllib.request.urlopen(f"{comfy}/history/{pid}", timeout=60))
        if pid in h:
            break
        time.sleep(3)
    st = h[pid]["status"]
    if st["status_str"] != "success":
        raise SystemExit("ComfyUI failed: " + json.dumps(st)[:600])
    for node in h[pid]["outputs"].values():
        for im in node.get("images", []):
            q = f"filename={im['filename']}&subfolder={im['subfolder']}&type={im['type']}"
            open(out, "wb").write(urllib.request.urlopen(f"{comfy}/view?{q}", timeout=120).read())
            return


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("n", type=int)
    ap.add_argument("seeds", type=int)
    ap.add_argument("outdir")
    ap.add_argument("--comfy", default="http://127.0.0.1:8188")
    ap.add_argument("--cast", default="")
    ap.add_argument("--rating", default="general")
    ap.add_argument("--first-seed", type=int, default=1000)
    a = ap.parse_args()
    os.makedirs(a.outdir, exist_ok=True)
    cast = a.cast.split("|") if a.cast else None
    for k in range(a.seeds):
        seed = a.first_seed + k
        out = os.path.join(a.outdir, f"cast{a.n}-seed{seed}.png")
        t = time.time()
        run(a.comfy, build(a.n, seed, cast, a.rating), out)
        print(out, f"{time.time() - t:.0f}s", flush=True)


if __name__ == "__main__":
    sys.exit(main())
