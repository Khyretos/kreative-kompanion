#!/usr/bin/env python3
"""BLD-03: live check. Ask the local Kompanion (test user session, AUTH-01) for a Blender scene and save the picture.
Usage: live_chat.py "prompt" out.png [Blender|"Blender (soucouyant)"]"""

import json
import os
import re
import sys
import time
import urllib.request

BASE = os.environ.get("KOMPANION_URL", "http://127.0.0.1:8095")
JAR = os.path.expanduser("~/.cache/kompanion-test-session.txt")
cookie = "; ".join(
    f"{l.split()[5]}={l.split()[6]}"
    for l in open(JAR)
    if l.strip()
    and (not l.startswith("#") or l.startswith("#HttpOnly_"))
    and len(l.split()) >= 7
)


def api(method, path, body=None, raw=False):
    req = urllib.request.Request(
        BASE + path,
        json.dumps(body).encode() if body is not None else None,
        {
            "Cookie": cookie,
            "X-Kompanion": "1",
            "Content-Type": "application/json",
            "Origin": BASE,
        },
        method=method,
    )
    data = urllib.request.urlopen(req, timeout=60).read()
    return data if raw else (json.loads(data) if data else None)


prompt, out = sys.argv[1], sys.argv[2]
tool = sys.argv[3] if len(sys.argv) > 3 else "Blender"
chat = api("POST", "/api/chats", {"title": "BLD-03 " + prompt[:30]})
api("PATCH", f"/api/chats/{chat['id']}", {"mcp": [tool]})
api("POST", f"/api/chats/{chat['id']}/messages", {"text": prompt})
last, stable = None, 0
for _ in range(240):
    time.sleep(3)
    msgs = [
        m
        for m in api("GET", f"/api/chats/{chat['id']}/messages")
        if m["author"] != "user"
    ]
    cur = msgs[-1]["text"] if msgs else ""
    stable = stable + 1 if cur and cur == last else 0
    last = cur
    if stable >= 4:
        break
print(last)
m = re.search(r"\(/api/chats/[^)]+\.png\)", last or "")
if m:
    open(out, "wb").write(api("GET", m.group(0)[1:-1], raw=True))
    print("[saved]", out)
