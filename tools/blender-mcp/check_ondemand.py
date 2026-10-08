#!/usr/bin/env python3
"""BLD-02: server.py started by systemd socket activation (LISTEN_FDS) exits by itself when idle.
Run: python3 tools/blender-mcp/check_ondemand.py"""
import json
import os
import socket
import subprocess
import sys
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ls = socket.socket(); ls.bind(("127.0.0.1", 0)); ls.listen(5); PORT = ls.getsockname()[1]
env = dict(os.environ, BLENDER_MCP_TOKEN="t0k", BLENDER_CMD=f"{sys.executable} {HERE}/stub_blender.py",
           LISTEN_FDS="1", LISTEN_PID="", BLENDER_IDLE_EXIT="2")
def pre():
    os.dup2(ls.fileno(), 3)
srv = subprocess.Popen([sys.executable, os.path.join(HERE, "server.py")], env=env, pass_fds=(3,), preexec_fn=pre)
fails = []
req = urllib.request.Request(f"http://127.0.0.1:{PORT}/mcp", json.dumps({"jsonrpc": "2.0", "id": 1, "method": "ping"}).encode(),
                             {"Content-Type": "application/json", "Authorization": "Bearer t0k"})
try:
    r = urllib.request.urlopen(req, timeout=10)
    if r.status != 200: fails.append("ping on the inherited socket")
    t0 = time.time()
    while srv.poll() is None and time.time() - t0 < 8:
        time.sleep(0.2)
    if srv.poll() != 0: fails.append("did not exit 0 after idle")
finally:
    if srv.poll() is None: srv.kill()
for f in fails: print("error: tools/blender-mcp/server.py:", f)
sys.exit(1 if fails else 0)
