#!/usr/bin/env python3
"""BLD-03: check for vision.py against a fake OpenAI-compatible server. Run: python3 tools/blender-mcp/check_vision.py"""
import base64, http.server, json, os, sys, threading
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
fails = []

def check(name, cond):
    if not cond:
        fails.append(name)
        print(f"error: tools/blender-mcp/vision.py: {name}")

SEEN = []
MODE = ["ok"]

class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        SEEN.append((self.path, self.headers.get("Authorization"), body))
        if MODE[0] == "500":
            self.send_response(500); self.end_headers(); return
        out = {"choices": [{"message": {"content": "  A red monkey head on a green floor.  "}}]}
        raw = json.dumps(out).encode()
        self.send_response(200); self.send_header("Content-Type", "application/json"); self.send_header("Content-Length", str(len(raw))); self.end_headers(); self.wfile.write(raw)

srv = http.server.HTTPServer(("127.0.0.1", 0), H)
threading.Thread(target=srv.serve_forever, daemon=True).start()
PORT = srv.server_address[1]
try:
    os.environ.pop("BLENDER_VISION_URL", None)
    try:
        import vision
    except Exception as e:
        print(f"error: tools/blender-mcp/vision.py: import: {e!r}"); sys.exit(1)
    png = b"\x89PNG fake"
    check("no url configured: empty string, no request", vision.describe(png, ["a"]) == "" and not SEEN)
    os.environ["BLENDER_VISION_URL"] = f"http://127.0.0.1:{PORT}/v1/"
    os.environ["BLENDER_VISION_MODEL"] = "qwen-test"
    os.environ["BLENDER_VISION_KEY"] = "sekret"
    t = vision.describe(png, ["monkey", "ground"])
    check("returns the stripped answer", t == "A red monkey head on a green floor.")
    path, auth, body = SEEN[-1]
    check("posts to /v1/chat/completions (trailing slash handled)", path == "/v1/chat/completions")
    check("bearer key sent", auth == "Bearer sekret")
    check("model", body["model"] == "qwen-test")
    check("no thinking", body.get("reasoning_effort") == "none")
    parts = body["messages"][-1]["content"]
    img = [p for p in parts if p.get("type") == "image_url"]
    txt = " ".join(p["text"] for p in parts if p.get("type") == "text")
    check("picture as a data URL", len(img) == 1 and img[0]["image_url"]["url"] == "data:image/png;base64," + base64.b64encode(png).decode())
    check("prompt names the planned objects", "monkey" in txt and "ground" in txt)
    check("prompt forbids guessing", "do not guess" in txt.lower() or "only what you see" in txt.lower())
    MODE[0] = "500"
    check("server error gives empty string", vision.describe(png, []) == "")
    os.environ["BLENDER_VISION_URL"] = "http://127.0.0.1:1/v1"
    check("unreachable gives empty string", vision.describe(png, []) == "")
finally:
    srv.shutdown()
print("ALL PASS" if not fails else f"{len(fails)} failed")
sys.exit(1 if fails else 0)
