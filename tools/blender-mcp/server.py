#!/usr/bin/env python3
"""BLD-01: a small MCP server (streamable HTTP, JSON answers) that runs Blender headless. Opt-in: run it only on a machine where chats may run Python inside Blender. Env: BLENDER_MCP_TOKEN (required), BLENDER_MCP_BIND (default 127.0.0.1:9876), BLENDER_CMD (default "blender"; e.g. "flatpak run --filesystem=/tmp org.blender.Blender"), BLENDER_TIMEOUT seconds (default 120), BLENDER_ENGINE (CYCLES, CPU with denoising; or EEVEE), BLENDER_SAMPLES (Cycles samples, default 128). Env: LISTEN_FDS=1 enables socket activation (binds fd 3); BLENDER_IDLE_EXIT=N exits after N seconds of inactivity."""
import base64, http.server, json, os, re, shutil, subprocess, sys, tempfile, uuid, time, socket, threading

TOKEN = os.environ.get("BLENDER_MCP_TOKEN", "")
HOST, _, PORT = os.environ.get("BLENDER_MCP_BIND", "127.0.0.1:9876").rpartition(":")
CMD = __import__("shlex").split(os.environ.get("BLENDER_CMD", "blender"))
TIMEOUT = int(os.environ.get("BLENDER_TIMEOUT", "120"))
DEFAULT_ENGINE = os.environ.get("BLENDER_ENGINE", "CYCLES")
HERE = os.path.dirname(os.path.abspath(__file__))
SCENE = open(os.path.join(HERE, "scene.py")).read()

SHAPES = ["cube", "sphere", "plane", "cylinder", "cone", "torus", "monkey"]
TEMPLATES = ["snowman", "character", "table"]
RELATIONS = ("on", "attached_to", "next_to")
SIDES = ["front", "back", "left", "right", "top", "bottom"]

def scene_code(spec):
    """BLD-03: the bpy line that builds a planned scene (see scene.py build()). ValueError on a bad plan."""
    if isinstance(spec, list):  # the old call shape: just a list of shapes
        spec = {"objects": spec}
    objects = list(spec.get("objects") or [])
    if not objects and not spec.get("template"):
        raise ValueError("give a template (" + ", ".join(TEMPLATES) + ") or a list of objects")
    if spec.get("template") and spec["template"] not in TEMPLATES:
        raise ValueError(f"unknown template {spec['template']}; use one of {', '.join(TEMPLATES)}")
    names = {"ground", "top", "bottom", "middle", "head"} if spec.get("template") else set()
    for i, o in enumerate(objects):
        shape = str(o.get("shape", ""))
        if shape not in SHAPES:
            raise ValueError(f"unknown shape {shape}; use one of {', '.join(SHAPES)}")
        for k in RELATIONS:
            if o.get(k) and str(o[k]) not in names and not spec.get("template"):
                raise ValueError(f"{o.get('name', shape)}: {k} '{o[k]}' is not an object listed before it; list the base object first")
        if o.get("side") and o["side"] not in SIDES:
            raise ValueError(f"unknown side {o['side']}; use one of {', '.join(SIDES)}")
        names.add(str(o.get("name") or f"{shape}_{i + 1}"))
    return "build(" + repr(spec) + ")"

PLAN = ("PLAN FIRST: list every object with a name, shape and size, and how it relates to the others. "
        "Parts that must touch (a snowman's balls, a head on a body, a monkey on a floor) use on / attached_to / next_to, never guessed coordinates; "
        "an object with no relation stands on the floor. List an object after the one it refers to. A floor, a camera aimed at everything and a sun are added.")
OBJECT = {"type": "object", "properties": {
    "name": {"type": "string", "description": "short unique name, e.g. body"},
    "shape": {"type": "string", "enum": SHAPES},
    "size": {"type": "number", "description": "width in metres; a sphere's diameter. Default 2"},
    "scale": {"type": "array", "items": {"type": "number"}, "description": "stretch [x, y, z], e.g. a flat slab [3, 2, 0.2]"},
    "color": {"description": "a name (red, blue, green, white, black, brown, yellow, orange, grey, skin, wood, snow) or [r, g, b] from 0 to 1"},
    "at": {"type": "array", "items": {"type": "number"}, "description": "[x, y] stands on the floor there; [x, y, z] puts it in the air"},
    "on": {"type": "string", "description": "name of the object this one rests on top of"},
    "attached_to": {"type": "string", "description": "name of the object this one is fixed onto (with side)"},
    "next_to": {"type": "string", "description": "name of the object this one stands beside (with side, gap)"},
    "side": {"type": "string", "enum": SIDES, "description": "which side of the other object; front is the side facing the camera"},
    "offset": {"type": "array", "items": {"type": "number"}, "description": "shift along the surface from the other object's centre (two numbers)"},
    "rotation": {"type": "array", "items": {"type": "number"}, "description": "degrees [x, y, z]"},
    "floating": {"type": "boolean", "description": "true only when it is meant to hang in the air"}},
    "required": ["shape"]}

TOOLS = [
    {"name": "blender_scene", "description": "Render a 3D scene from a plan and show the picture. Prefer this tool; it cannot fail on code. " + PLAN + " Use a template ONLY when the user asks for exactly that subject: 'snowman', 'character' (a person or cartoon figure) or 'table' (size = its height; extra objects sit on it with on 'top'; never re-list the template's own parts). The floor is always there: never add your own plane or ground, just place things on it (they stand on it by default; on 'ground' also works). A 'plane' is a flat floor, never an aircraft. Examples: {'template': 'table', 'objects': [{'name': 'ball', 'shape': 'sphere', 'size': 0.5, 'color': 'red', 'on': 'top'}]}; {'objects': [{'name': 'monkey', 'shape': 'monkey', 'size': 2, 'color': 'red'}]} (stands on the floor). Never use the 'location' key. Example: {\"objects\": [{\"name\": \"monkey\", \"shape\": \"monkey\", \"size\": 2, \"color\": \"red\"}]} or {\"template\": \"snowman\"}. The answer lists measured facts (sizes, what touches what, WARNING lines) and what a vision model saw in the render: describe the render ONLY from that and the picture, and call again to fix every WARNING.",
     "inputSchema": {"type": "object", "properties": {"objects": {"type": "array", "items": OBJECT}, "template": {"type": "string", "enum": TEMPLATES}, "size": {"type": "number", "description": "template height in metres, default 3"}, "ground": {"type": "boolean", "description": "floor on (default) or off"}, "width": {"type": "integer"}, "height": {"type": "integer"}}}},
    {"name": "blender_run", "description": "Run Python (bpy) in a fresh headless Blender and return its printed output. Use print() for answers.", "inputSchema": {"type": "object", "properties": {"code": {"type": "string"}}, "required": ["code"]}},
    {"name": "blender_render", "description": "Only for what blender_scene cannot do: build a scene with Python (bpy) and render it. The scene starts empty; a camera and sun are added. Use the helpers, not raw bpy.ops: add(shape, size, at, color, name, scale, rotation) returns the object; put_on_ground(o); put_on(o, base, offset=(dx,dy), sink=0); stack([a, b, c]); attach(o, to, side='front', offset=(a,b), sink=0.25); next_to(o, other, side='right'); ground_plane(); face_camera(o). Colours are names ('red') or (r, g, b). Example code: ground_plane(); body = add('sphere', 2, color='white'); put_on_ground(body); head = add('sphere', 1.2, color='white'); put_on(head, body, sink=0.1); nose = add('cone', 0.2, color='orange', rotation=(90, 0, 0)); attach(nose, head, 'front'). The answer lists measured facts and what a vision model saw; describe the render only from that and the picture.", "inputSchema": {"type": "object", "properties": {"code": {"type": "string"}, "width": {"type": "integer"}, "height": {"type": "integer"}, "engine": {"type": "string", "enum": ["CYCLES", "EEVEE"]}}, "required": ["code"]}},
]

def blender(script, d=None):
    if d is None:
        d = tempfile.mkdtemp(prefix="kk-blender-", dir="/tmp")
    path = os.path.join(d, "script.py")
    with open(path, "w") as f:
        f.write(script)
    try:
        r = subprocess.run(CMD + ["-b", "--factory-startup", "--python-exit-code", "1", "--python", path], capture_output=True, text=True, timeout=TIMEOUT)
        ok = r.returncode == 0
        log = (r.stdout + r.stderr)[-4000:]
    except subprocess.TimeoutExpired:
        ok = False
        log = f"Blender timed out after {TIMEOUT} s"
    return ok, log, d

def seen(png, report):
    """What a vision model saw in the render (BLD-03), or the reason there is none. Never a made-up description."""
    try:
        from vision import describe
        names = re.findall(r"^- ([^:]+):", report, re.M)
        text = describe(png, names)
    except Exception:
        text = ""
    return text or "(no picture check available; describe only the measured facts above and the picture itself)"

def failure(log, code):
    try:
        from errors import explain_error
        return explain_error(log, code)
    except Exception:
        return log

def call(name, args):
    if os.environ.get("BLENDER_DEBUG"):
        print("call", name, json.dumps(args)[:1500], flush=True)
    if name == "blender_scene":
        try:
            spec = {k: args[k] for k in ("objects", "template", "size", "ground") if k in args}
            code = scene_code(spec)
        except (ValueError, TypeError, AttributeError) as e:
            return {"content": [{"type": "text", "text": str(e)}], "isError": True}
        return call("blender_render", {"code": code, "width": args.get("width", 640), "height": args.get("height", 480)})
    if name == "blender_run":
        script = "import bpy\n" + args.get("code", "")
        ok, log, d = blender(script)
        shutil.rmtree(d, ignore_errors=True)
        return {"content": [{"type": "text", "text": log}], "isError": not ok}
    if name == "blender_render":
        w = int(args.get("width", 640))
        h = int(args.get("height", 480))
        eng = str(args.get("engine", DEFAULT_ENGINE))
        d = tempfile.mkdtemp(prefix="kk-blender-", dir="/tmp")
        out = os.path.join(d, "render.png")
        code = args.get("code", "")
        script = (SCENE + "\nprepare()\n"
                  + "exec(compile(" + repr(code) + ", 'chat_code', 'exec'), dict(globals(), __name__='__main__'))\n"
                  + "finish(" + repr(out) + ", " + str(w) + ", " + str(h) + ", " + repr(eng) + ")\n")
        ok, log, d = blender(script, d=d)
        png = open(out, "rb").read() if ok and os.path.exists(out) else None
        blend_path = os.path.splitext(out)[0] + ".blend"
        blend = open(blend_path, "rb").read() if png and os.path.exists(blend_path) else None
        txt_path = os.path.splitext(out)[0] + ".txt"
        report = open(txt_path).read() if png and os.path.exists(txt_path) else ""
        shutil.rmtree(d, ignore_errors=True)
        if png:
            text = "Rendered " + str(w) + "x" + str(h) + "."
            if report:
                text += ("\nMeasured from the scene (sizes in metres):\n" + report + "\nWhat a vision model saw in the picture:\n" + seen(png, report)
                         + "\nDescribe the render only from this and the picture. Do not add details nobody listed. For what touches or floats, the measured facts win over the picture check.")
            content = [{"type": "text", "text": text}, {"type": "image", "mimeType": "image/png", "data": base64.b64encode(png).decode()}]
            if blend:
                # CHAT-05: the .blend as an embedded MCP resource; the chat offers it as a download.
                content.append({"type": "resource", "resource": {"uri": "file:///scene.blend", "mimeType": "application/x-blender", "blob": base64.b64encode(blend).decode()}})
            return {"content": content, "isError": False}
        return {"content": [{"type": "text", "text": failure(log, code)}], "isError": True}
    return {"content": [{"type": "text", "text": f"unknown tool {name}"}], "isError": True}

LAST = [time.time()]
BUSY = [0]

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self): self.send_error(405)
    def do_DELETE(self): self.send_error(405)
    def _post(self):
        if self.path != "/mcp": self.send_error(404); return
        auth = self.headers.get("Authorization")
        if not TOKEN or auth != "Bearer " + TOKEN:
            self.send_response(401); self.end_headers(); return
        cl = int(self.headers.get("Content-Length", 0))
        body = json.loads(self.rfile.read(cl).decode()) if cl else {}
        try:
            id_ = body.get("id")
            method = body.get("method")
            if "id" not in body:
                self.send_response(202); self.send_header("Content-Length", "0"); self.end_headers(); return
            if method == "initialize":
                sid = uuid.uuid4().hex
                res = {"protocolVersion": "2025-03-26", "capabilities": {"tools": {}}, "serverInfo": {"name": "blender", "version": "0.1.0"}}
                self.send_response(200); self.send_header("Content-Type", "application/json"); self.send_header("Mcp-Session-Id", sid); self.end_headers()
                self.wfile.write(json.dumps({"jsonrpc": "2.0", "id": id_, "result": res}).encode())
            elif method == "tools/list":
                self.send_response(200); self.send_header("Content-Type", "application/json"); self.end_headers()
                self.wfile.write(json.dumps({"jsonrpc": "2.0", "id": id_, "result": {"tools": TOOLS}}).encode())
            elif method == "tools/call":
                params = body.get("params", {})
                res = call(params["name"], params.get("arguments") or {})
                self.send_response(200); self.send_header("Content-Type", "application/json"); self.end_headers()
                self.wfile.write(json.dumps({"jsonrpc": "2.0", "id": id_, "result": res}).encode())
            elif method == "ping":
                self.send_response(200); self.send_header("Content-Type", "application/json"); self.end_headers()
                self.wfile.write(json.dumps({"jsonrpc": "2.0", "id": id_, "result": {}}).encode())
            else:
                self.send_response(200); self.send_header("Content-Type", "application/json"); self.end_headers()
                self.wfile.write(json.dumps({"jsonrpc": "2.0", "id": id_, "error": {"code": -32601, "message": "unknown method"}}).encode())
        except Exception:
            self.send_response(500); self.send_header("Content-Type", "application/json"); self.end_headers()
            self.wfile.write(json.dumps({"jsonrpc": "2.0", "id": id_, "error": {"code": -32603, "message": "internal error"}}).encode())

    def do_POST(self):
        BUSY[0] += 1
        try:
            self._post()
        finally:
            BUSY[0] -= 1
            LAST[0] = time.time()

if __name__ == "__main__":
    if not TOKEN: print("set BLENDER_MCP_TOKEN", file=sys.stderr); sys.exit(2)
    idle_exit = int(os.environ.get("BLENDER_IDLE_EXIT", "0"))
    if os.environ.get("LISTEN_FDS") == "1":
        srv = http.server.ThreadingHTTPServer((HOST, 0), Handler, bind_and_activate=False)
        srv.socket = socket.socket(fileno=3)
        srv.server_address = srv.socket.getsockname()
    else:
        srv = http.server.ThreadingHTTPServer((HOST, int(PORT)), Handler)
    if idle_exit > 0:
        def check_idle():
            while True:
                time.sleep(0.5)
                if BUSY[0] == 0 and time.time() - LAST[0] > idle_exit:
                    os._exit(0)
        t = threading.Thread(target=check_idle, daemon=True)
        t.start()
    srv.serve_forever()
