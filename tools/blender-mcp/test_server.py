#!/usr/bin/env python3
"""BLD-01: server.py against a fake Blender. Run: python3 tools/blender-mcp/test_server.py"""

import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
s = socket.socket()
s.bind(("127.0.0.1", 0))
PORT = s.getsockname()[1]
s.close()
env = dict(
    os.environ,
    BLENDER_MCP_TOKEN="t0k",
    BLENDER_MCP_BIND=f"127.0.0.1:{PORT}",
    BLENDER_CMD=f"{sys.executable} {HERE}/stub_blender.py",
    BLENDER_TIMEOUT="5",
)
srv = subprocess.Popen([sys.executable, os.path.join(HERE, "server.py")], env=env)
fails = []


def check(name, cond):
    if not cond:
        fails.append(name)
        print(f"error: tools/blender-mcp/server.py: {name}")


def post(body, token="t0k", sid=None):
    h = {
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
    }
    if token:
        h["Authorization"] = f"Bearer {token}"
    if sid:
        h["Mcp-Session-Id"] = sid
    req = urllib.request.Request(
        f"http://127.0.0.1:{PORT}/mcp", json.dumps(body).encode(), h
    )
    try:
        r = urllib.request.urlopen(req, timeout=20)
        raw = r.read().decode()
        return r.status, dict(r.headers), (json.loads(raw) if raw else None)
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), None


try:
    for _ in range(50):
        try:
            socket.create_connection(("127.0.0.1", PORT), 0.2).close()
            break
        except OSError:
            time.sleep(0.1)
    with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=5) as r:
        check("health 200 without a token", r.status == 200 and r.read() == b"ok")
    check(
        "no token gives 401",
        post({"jsonrpc": "2.0", "id": 1, "method": "initialize"}, token=None)[0] == 401,
    )
    check(
        "wrong token gives 401",
        post({"jsonrpc": "2.0", "id": 1, "method": "initialize"}, token="no")[0] == 401,
    )
    st, h, b = post({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
    sid = h.get("Mcp-Session-Id") or h.get("mcp-session-id")
    check("initialize 200", st == 200)
    check("initialize session id", bool(sid))
    check(
        "initialize result",
        b
        and b["id"] == 1
        and b["result"]["serverInfo"]["name"] == "blender"
        and "tools" in b["result"]["capabilities"],
    )
    st, _, b = post({"jsonrpc": "2.0", "method": "notifications/initialized"}, sid=sid)
    check("notification 202", st == 202 and b is None)
    st, _, b = post({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}, sid=sid)
    names = [t["name"] for t in b["result"]["tools"]] if b else []
    check("three tools", names == ["blender_scene", "blender_run", "blender_render"])
    if b and len(b["result"]["tools"]) == 3:
        r = b["result"]["tools"][2]["inputSchema"]
        sc = b["result"]["tools"][0]["inputSchema"]
        check(
            "scene schema",
            sc["properties"]["objects"]["type"] == "array"
            and sc["properties"]["template"]["enum"]
            == ["snowman", "character", "table"]
            and sc["properties"]["objects"]["items"]["properties"]["shape"]["enum"]
            == ["cube", "sphere", "plane", "cylinder", "cone", "torus", "monkey"],
        )
        check(
            "render schema",
            r["required"] == ["code"]
            and set(r["properties"]) >= {"code", "width", "height", "engine"},
        )
    st, _, b = post(
        {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {"name": "blender_run", "arguments": {"code": "print(1)"}},
        },
        sid=sid,
    )
    check(
        "run ok",
        b
        and b["id"] == 3
        and b["result"].get("isError") is False
        and "stub blender" in b["result"]["content"][0]["text"],
    )
    st, _, b = post(
        {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {"name": "blender_run", "arguments": {"code": "RAISE"}},
        },
        sid=sid,
    )
    check(
        "run error",
        b
        and b["result"].get("isError") is True
        and "ValueError: RAISE" in b["result"]["content"][0]["text"],
    )
    st, _, b = post(
        {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "blender_render",
                "arguments": {
                    "code": "bpy.ops.mesh.primitive_monkey_add()",
                    "width": 64,
                    "height": 48,
                },
            },
        },
        sid=sid,
    )
    c = b["result"]["content"] if b else []
    img = [x for x in c if x.get("type") == "image"]
    res = [x for x in c if x.get("type") == "resource"]
    check(
        "render blend",
        len(res) == 1
        and res[0]["resource"]["mimeType"] == "application/x-blender"
        and res[0]["resource"]["uri"].endswith("scene.blend")
        and __import__("base64").b64decode(res[0]["resource"]["blob"])
        == b"BLENDER-v430",
    )
    check(
        "render image",
        len(img) == 1
        and img[0]["mimeType"] == "image/png"
        and img[0]["data"].startswith("iVBORw0KGgo"),
    )
    check("render text first", bool(c) and c[0].get("type") == "text")
    st, _, b = post(
        {
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {"name": "blender_render", "arguments": {"code": "RAISE"}},
        },
        sid=sid,
    )
    check(
        "render error has no image",
        b
        and b["result"].get("isError") is True
        and all(x["type"] == "text" for x in b["result"]["content"]),
    )
    st, _, b = post(
        {
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {"name": "blender_run", "arguments": {"code": "SLEEP"}},
        },
        sid=sid,
    )
    check(
        "timeout is an error",
        b
        and b["result"].get("isError") is True
        and "timed out" in b["result"]["content"][0]["text"],
    )
    st, _, b = post(
        {
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "blender_scene",
                "arguments": {
                    "objects": [{"name": "box", "shape": "cube", "color": "red"}]
                },
            },
        },
        sid=sid,
    )
    c = b["result"]["content"] if b else []
    check(
        "scene renders",
        b
        and b["result"].get("isError") is False
        and any(x.get("type") == "image" for x in c),
    )
    t = c[0]["text"] if c else ""
    check(
        "scene text carries the measured facts",
        "Measured from the scene" in t and "- box:" in t,
    )
    check(
        "scene text never invents a picture description",
        "no picture check available" in t and "Do not add details" in t,
    )
    st, _, b = post(
        {
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "blender_scene",
                "arguments": {"objects": [{"shape": "teapot"}]},
            },
        },
        sid=sid,
    )
    check(
        "unknown shape",
        b
        and b["result"].get("isError") is True
        and "unknown shape teapot" in b["result"]["content"][0]["text"],
    )
    st, _, b = post({"jsonrpc": "2.0", "id": 8, "method": "nope"}, sid=sid)
    check("unknown method", b and b["error"]["code"] == -32601)
    st, _, b = post(
        {
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {"name": "nope", "arguments": {}},
        },
        sid=sid,
    )
    check("unknown tool", b and ("error" in b or b["result"].get("isError") is True))
    # scene_code: a plan becomes one build() call (server.py imported directly)
    sys.path.insert(0, HERE)
    try:
        import server
    except Exception as e:
        check("import server: " + repr(e), False)
        raise SystemExit(1)
    code = server.scene_code(
        {
            "objects": [
                {"name": "body", "shape": "sphere", "size": 2},
                {"name": "head", "shape": "sphere", "on": "body"},
            ]
        }
    )
    check(
        "scene_code is a build call " + code,
        code.startswith("build(") and "'on': 'body'" in code,
    )
    check(
        "scene_code template",
        server.scene_code({"template": "snowman"}).startswith("build("),
    )
    check(
        "scene_code old list shape",
        server.scene_code([{"shape": "cube"}]).startswith("build("),
    )
    for bad, msg in [
        (
            {"objects": [{"shape": "teapot"}]},
            "unknown shape teapot; use one of cube, sphere, plane",
        ),
        (
            {"objects": [{"shape": "sphere", "on": "body"}]},
            "on 'body' is not an object listed before it",
        ),
        (
            {"objects": [{"shape": "sphere", "attached_to": "x", "side": "up"}]},
            "is not an object listed before it",
        ),
        ({"template": "castle"}, "unknown template castle"),
        ({}, "give a template"),
    ]:
        try:
            server.scene_code(bad)
            check("scene_code accepted " + repr(bad), False)
        except ValueError as e:
            check("scene_code error " + str(e), msg in str(e))
    ex = server.failure("noise", "x")
    check("failure never raises", isinstance(ex, str))
finally:
    srv.kill()
print("ALL PASS" if not fails else f"{len(fails)} failed")
sys.exit(1 if fails else 0)
