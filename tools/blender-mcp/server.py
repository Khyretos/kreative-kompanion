#!/usr/bin/env python3
"""BLD-01: a small MCP server (streamable HTTP, JSON answers) that runs Blender headless. Opt-in: run it only on a machine where chats may run Python inside Blender. Env: BLENDER_MCP_TOKEN (required), BLENDER_MCP_BIND (default 127.0.0.1:9876), BLENDER_CMD (default "blender"; e.g. "flatpak run --filesystem=/tmp org.blender.Blender"), BLENDER_TIMEOUT seconds (default 120)."""
import base64, http.server, json, os, shutil, subprocess, sys, tempfile, uuid

TOKEN = os.environ.get("BLENDER_MCP_TOKEN", "")
HOST, _, PORT = os.environ.get("BLENDER_MCP_BIND", "127.0.0.1:9876").rpartition(":")
CMD = __import__("shlex").split(os.environ.get("BLENDER_CMD", "blender"))
TIMEOUT = int(os.environ.get("BLENDER_TIMEOUT", "120"))
HERE = os.path.dirname(os.path.abspath(__file__))
SCENE = open(os.path.join(HERE, "scene.py")).read()

SHAPES = {
    "cube": "bpy.ops.mesh.primitive_cube_add(size={s}, location={l})",
    "sphere": "bpy.ops.mesh.primitive_uv_sphere_add(radius={h}, location={l})",
    "plane": "bpy.ops.mesh.primitive_plane_add(size={s}, location={l})",
    "cylinder": "bpy.ops.mesh.primitive_cylinder_add(radius={h}, depth={s}, location={l})",
    "cone": "bpy.ops.mesh.primitive_cone_add(radius1={h}, depth={s}, location={l})",
    "torus": "bpy.ops.mesh.primitive_torus_add(major_radius={h}, minor_radius={e}, location={l})",
    "monkey": "bpy.ops.mesh.primitive_monkey_add(size={s}, location={l})",
}

def scene_code(objects):
    """The bpy lines for a list of shapes; ValueError on an unknown shape."""
    lines = []
    for o in objects:
        shape = str(o.get("shape", ""))
        if shape not in SHAPES:
            raise ValueError(f"unknown shape {shape}; use one of {', '.join(SHAPES)}")
        s = float(o.get("size", 2))
        loc = tuple(float(v) for v in (list(o.get("location") or [0, 0, 0]) + [0, 0, 0])[:3])
        lines.append(SHAPES[shape].format(s=s, h=s / 2, e=s / 8, l=loc))
        if o.get("color"):
            lines.append(f"color(bpy.context.active_object, {tuple(float(v) for v in o['color'])})")
        if o.get("rotation"):
            x, y, z = (float(v) for v in (list(o["rotation"]) + [0, 0, 0])[:3])
            lines.append(f"bpy.context.active_object.rotation_euler = (math.radians({x}), math.radians({y}), math.radians({z}))")
    return "\n".join(lines)

TOOLS = [
    {"name": "blender_scene", "description": "Render a simple 3D scene from a list of shapes and show the picture to the user. Prefer this tool. A camera aimed at everything and a sun light are added for you. Example: objects [{\"shape\": \"plane\", \"size\": 8}, {\"shape\": \"cube\", \"location\": [0, 0, 1], \"color\": [0.8, 0.1, 0.1]}]", "inputSchema": {"type": "object", "properties": {"objects": {"type": "array", "items": {"type": "object", "properties": {"shape": {"type": "string", "enum": ["cube", "sphere", "plane", "cylinder", "cone", "torus", "monkey"]}, "size": {"type": "number", "description": "Width in metres, default 2"}, "location": {"type": "array", "items": {"type": "number"}, "description": "[x, y, z]; z is up"}, "rotation": {"type": "array", "items": {"type": "number"}, "description": "[x, y, z] in degrees"}, "color": {"type": "array", "items": {"type": "number"}, "description": "[r, g, b], each 0..1"}}, "required": ["shape"]}}, "width": {"type": "integer", "default": 640}, "height": {"type": "integer", "default": 480}}, "required": ["objects"]}},
    {"name": "blender_run", "description": "Run Python (bpy) in a fresh headless Blender and return its printed output. Use print() for answers.", "inputSchema": {"type": "object", "properties": {"code": {"type": "string", "description": "Python code using bpy"}}, "required": ["code"]}},
    {"name": "blender_render", "description": "Build a scene with Python (bpy) and render it to a picture the user sees. The scene starts empty. Add only objects and colours: a camera aimed at everything and a sun light are added for you, so do not add your own unless the user asks. color(obj, (r, g, b)) gives an object a coloured material. Example code: bpy.ops.mesh.primitive_plane_add(size=6); bpy.ops.mesh.primitive_cube_add(location=(0, 0, 1)); color(bpy.context.active_object, (0.8, 0.1, 0.1))", "inputSchema": {"type": "object", "properties": {"code": {"type": "string"}, "width": {"type": "integer", "default": 640}, "height": {"type": "integer", "default": 480}, "engine": {"type": "string", "enum": ["EEVEE", "CYCLES"], "default": "EEVEE"}}, "required": ["code"]}}
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

def call(name, args):
    if name == "blender_scene":
        try:
            code = scene_code(args.get("objects") or [])
        except (ValueError, TypeError) as e:
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
        eng = str(args.get("engine", "EEVEE"))
        d = tempfile.mkdtemp(prefix="kk-blender-", dir="/tmp")
        out = os.path.join(d, "render.png")
        code = args.get("code", "")
        script = (SCENE + "\nprepare()\n"
                  + "exec(compile(" + repr(code) + ", 'chat_code', 'exec'), {'bpy': bpy, 'color': color, 'math': math, 'Vector': Vector, '__name__': '__main__'})\n"
                  + "finish(" + repr(out) + ", " + str(w) + ", " + str(h) + ", " + repr(eng) + ")\n")
        ok, log, d = blender(script, d=d)
        png = open(out, "rb").read() if ok and os.path.exists(out) else None
        shutil.rmtree(d, ignore_errors=True)
        if png:
            return {"content": [{"type": "text", "text": "Rendered " + str(w) + "x" + str(h) + "."}, {"type": "image", "mimeType": "image/png", "data": base64.b64encode(png).decode()}], "isError": False}
        return {"content": [{"type": "text", "text": log}], "isError": True}
    return {"content": [{"type": "text", "text": f"unknown tool {name}"}], "isError": True}

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self): self.send_error(405)
    def do_DELETE(self): self.send_error(405)
    def do_POST(self):
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

if __name__ == "__main__":
    if not TOKEN: print("set BLENDER_MCP_TOKEN", file=sys.stderr); sys.exit(2)
    http.server.ThreadingHTTPServer((HOST, int(PORT)), Handler).serve_forever()
