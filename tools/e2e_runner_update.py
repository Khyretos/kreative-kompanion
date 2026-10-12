"""RUNNER-SYNC end-to-end check.

A fake Kompanion server queues a self_update job; the runner given as argv[1] must
download, verify and install the served binary, report the job and exit with status 4.
Usage: python3 tools/e2e_runner_update.py runner/target/debug/kompanion-runner
"""

import hashlib
import http.server
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading

d = tempfile.mkdtemp(prefix="kk-e2e-")
new = b"#!/bin/sh\necho kompanion-runner 9.9.9\n"
results = []


class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(n)
        if self.path.endswith("/stats"):
            out = (
                json.dumps(
                    {
                        "interval": 1,
                        "jobs": [{"id": "job1", "tool": {"tool": "self_update"}}],
                    }
                ).encode()
                if not [r for r in results if r.get("job_id") == "job1"]
                else json.dumps({"interval": 1, "jobs": []}).encode()
            )
        else:
            results.append(json.loads(body))
            out = b"{}"
        self.send_response(200)
        self.send_header("Content-Length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

    def do_GET(self):
        if self.path == "/download/kompanion-runner":
            out = new
        elif self.path == "/download/kompanion-runner.sha256":
            out = (hashlib.sha256(new).hexdigest() + "  kompanion-runner\n").encode()
        else:
            self.send_response(404)
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Content-Length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)


srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), H)
port = srv.server_address[1]
threading.Thread(target=srv.serve_forever, daemon=True).start()
home = os.path.join(d, "home")
os.makedirs(home + "/.config/kompanion-runner")
exe = os.path.join(d, "kompanion-runner")
shutil.copy(sys.argv[1], exe)
open(home + "/.config/kompanion-runner/token", "w").write("tok\n")
open(home + "/.config/kompanion-runner/grants.json", "w").write("[]")
conf = home + "/.config/kompanion-runner"
open(conf + "/config.toml", "w").write(
    f'server = "http://127.0.0.1:{port}"\n'
    'machine_id = "m1"\n'
    f'token_file = "{conf}/token"\n'
    f'grants_file = "{conf}/grants.json"\n'
)
before = subprocess.run(
    [exe, "--version"], capture_output=True, text=True
).stdout.strip()
p = subprocess.Popen(
    [exe],
    env={"HOME": home, "PATH": os.environ["PATH"]},
    stderr=subprocess.PIPE,
    text=True,
)
try:
    rc = p.wait(timeout=30)
except subprocess.TimeoutExpired:
    p.kill()
    rc = "timeout"
after = subprocess.run(
    [exe, "--version"], capture_output=True, text=True
).stdout.strip()
res = [r for r in results if r.get("job_id") == "job1"]
print(
    "before:",
    before,
    "| after:",
    after,
    "| exit:",
    rc,
    "| result:",
    res[0]["ok"] if res else None,
    res[0]["output"] if res else "",
)
ok = rc == 4 and after == "kompanion-runner 9.9.9" and res and res[0]["ok"] is True
print("E2E", "PASS" if ok else "FAIL")
sys.exit(0 if ok else 1)
