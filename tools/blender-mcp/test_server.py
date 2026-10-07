#!/usr/bin/env python3
"""BLD-01: server.py against a fake Blender. Run: python3 tools/blender-mcp/test_server.py"""
import json, os, socket, subprocess, sys, time, urllib.request, urllib.error

HERE = os.path.dirname(os.path.abspath(__file__))
s = socket.socket(); s.bind(("127.0.0.1", 0)); PORT = s.getsockname()[1]; s.close()
env = dict(os.environ, BLENDER_MCP_TOKEN="t0k", BLENDER_MCP_BIND=f"127.0.0.1:{PORT}",
           BLENDER_CMD=f"{sys.executable} {HERE}/stub_blender.py", BLENDER_TIMEOUT="5")
srv = subprocess.Popen([sys.executable, os.path.join(HERE, "server.py")], env=env)
fails = []

def check(name, cond):
    if not cond:
        fails.append(name)
        print(f"error: tools/blender-mcp/server.py: {name}")

def post(body, token="t0k", sid=None):
    h = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
    if token: h["Authorization"] = f"Bearer {token}"
    if sid: h["Mcp-Session-Id"] = sid
    req = urllib.request.Request(f"http://127.0.0.1:{PORT}/mcp", json.dumps(body).encode(), h)
    try:
        r = urllib.request.urlopen(req, timeout=20)
        raw = r.read().decode()
        return r.status, dict(r.headers), (json.loads(raw) if raw else None)
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), None

try:
    for _ in range(50):
        try:
            socket.create_connection(("127.0.0.1", PORT), 0.2).close(); break
        except OSError:
            time.sleep(0.1)
    check("no token gives 401", post({"jsonrpc": "2.0", "id": 1, "method": "initialize"}, token=None)[0] == 401)
    check("wrong token gives 401", post({"jsonrpc": "2.0", "id": 1, "method": "initialize"}, token="no")[0] == 401)
    st, h, b = post({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
    sid = h.get("Mcp-Session-Id") or h.get("mcp-session-id")
    check("initialize 200", st == 200)
    check("initialize session id", bool(sid))
    check("initialize result", b and b["id"] == 1 and b["result"]["serverInfo"]["name"] == "blender" and "tools" in b["result"]["capabilities"])
    st, _, b = post({"jsonrpc": "2.0", "method": "notifications/initialized"}, sid=sid)
    check("notification 202", st == 202 and b is None)
    st, _, b = post({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}, sid=sid)
    names = [t["name"] for t in b["result"]["tools"]] if b else []
    check("three tools", names == ["blender_scene", "blender_run", "blender_render"])
    if b and len(b["result"]["tools"]) == 3:
        r = b["result"]["tools"][2]["inputSchema"]
        sc = b["result"]["tools"][0]["inputSchema"]
        check("scene schema", sc["required"] == ["objects"] and sc["properties"]["objects"]["type"] == "array" and sc["properties"]["objects"]["items"]["properties"]["shape"]["enum"] == ["cube", "sphere", "plane", "cylinder", "cone", "torus", "monkey"])
        check("render schema", r["required"] == ["code"] and set(r["properties"]) >= {"code", "width", "height", "engine"})
    st, _, b = post({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "blender_run", "arguments": {"code": "print(1)"}}}, sid=sid)
    check("run ok", b and b["id"] == 3 and b["result"].get("isError") is False and "stub blender" in b["result"]["content"][0]["text"])
    st, _, b = post({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "blender_run", "arguments": {"code": "RAISE"}}}, sid=sid)
    check("run error", b and b["result"].get("isError") is True and "ValueError: RAISE" in b["result"]["content"][0]["text"])
    st, _, b = post({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "blender_render", "arguments": {"code": "bpy.ops.mesh.primitive_monkey_add()", "width": 64, "height": 48}}}, sid=sid)
    c = b["result"]["content"] if b else []
    img = [x for x in c if x.get("type") == "image"]
    res = [x for x in c if x.get("type") == "resource"]
    check("render blend", len(res) == 1 and res[0]["resource"]["mimeType"] == "application/x-blender" and res[0]["resource"]["uri"].endswith("scene.blend") and __import__("base64").b64decode(res[0]["resource"]["blob"]) == b"BLENDER-v430")
    check("render image", len(img) == 1 and img[0]["mimeType"] == "image/png" and img[0]["data"].startswith("iVBORw0KGgo"))
    check("render text first", bool(c) and c[0].get("type") == "text")
    st, _, b = post({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"name": "blender_render", "arguments": {"code": "RAISE"}}}, sid=sid)
    check("render error has no image", b and b["result"].get("isError") is True and all(x["type"] == "text" for x in b["result"]["content"]))
    st, _, b = post({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": "blender_run", "arguments": {"code": "SLEEP"}}}, sid=sid)
    check("timeout is an error", b and b["result"].get("isError") is True and "timed out" in b["result"]["content"][0]["text"])
    st, _, b = post({"jsonrpc": "2.0", "id": 10, "method": "tools/call", "params": {"name": "blender_scene", "arguments": {"objects": [{"shape": "plane", "size": 8}, {"shape": "cube", "location": [0, 0, 1], "color": [1, 0, 0]}]}}}, sid=sid)
    c = b["result"]["content"] if b else []
    check("scene renders", b and b["result"].get("isError") is False and any(x.get("type") == "image" for x in c))
    st, _, b = post({"jsonrpc": "2.0", "id": 11, "method": "tools/call", "params": {"name": "blender_scene", "arguments": {"objects": [{"shape": "teapot"}]}}}, sid=sid)
    check("unknown shape", b and b["result"].get("isError") is True and "unknown shape teapot" in b["result"]["content"][0]["text"])
    st, _, b = post({"jsonrpc": "2.0", "id": 8, "method": "nope"}, sid=sid)
    check("unknown method", b and b["error"]["code"] == -32601)
    st, _, b = post({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {"name": "nope", "arguments": {}}}, sid=sid)
    check("unknown tool", b and ("error" in b or b["result"].get("isError") is True))
    # scene_code: the shapes become bpy lines (server.py imported directly)
    sys.path.insert(0, HERE)
    try:
        import server
    except Exception as e:
        check("import server: " + repr(e), False); raise SystemExit(1)
    if not hasattr(server, "scene_code"):
        check("no function scene_code(objects)", False); raise SystemExit(1)
    code = server.scene_code([{"shape": "plane", "size": 8}, {"shape": "cube", "location": [0, 0, 1], "color": [1, 0, 0], "rotation": [0, 0, 45]},
                              {"shape": "sphere", "size": 1}, {"shape": "cylinder"}, {"shape": "cone"}, {"shape": "torus"}, {"shape": "monkey"}])
    want = ["bpy.ops.mesh.primitive_plane_add(size=8.0, location=(0.0, 0.0, 0.0))",
            "bpy.ops.mesh.primitive_cube_add(size=2.0, location=(0.0, 0.0, 1.0))",
            "color(bpy.context.active_object, (1.0, 0.0, 0.0))",
            "bpy.context.active_object.rotation_euler = (math.radians(0.0), math.radians(0.0), math.radians(45.0))",
            "bpy.ops.mesh.primitive_uv_sphere_add(radius=0.5, location=(0.0, 0.0, 0.0))",
            "bpy.ops.mesh.primitive_cylinder_add(radius=1.0, depth=2.0, location=(0.0, 0.0, 0.0))",
            "bpy.ops.mesh.primitive_cone_add(radius1=1.0, depth=2.0, location=(0.0, 0.0, 0.0))",
            "bpy.ops.mesh.primitive_torus_add(major_radius=1.0, minor_radius=0.25, location=(0.0, 0.0, 0.0))",
            "bpy.ops.mesh.primitive_monkey_add(size=2.0, location=(0.0, 0.0, 0.0))"]
    check("scene_code lines " + repr(code.splitlines()), code.splitlines() == want)
    try:
        server.scene_code([{"shape": "teapot"}]); check("scene_code raises ValueError on an unknown shape", False)
    except ValueError as e:
        check("scene_code error text", str(e).startswith("unknown shape teapot; use one of cube, sphere, plane"))
finally:
    srv.kill()
print("ALL PASS" if not fails else f"{len(fails)} failed")
sys.exit(1 if fails else 0)
