#!/usr/bin/env python3
"""BLD-03: check for errors.py. Run: python3 tools/blender-mcp/check_errors.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
fails = []

def check(name, cond):
    if not cond:
        fails.append(name)
        print(f"error: tools/blender-mcp/errors.py: {name}")

try:
    from errors import explain_error
except Exception as e:
    print(f"error: tools/blender-mcp/errors.py: import: {e!r}"); sys.exit(1)

code = "bpy.ops.mesh.primitive_cube_add(size=2)\nbpy.ops.mesh.primitive_cube_add(radius=1)\nprint('x')"
log = '''Blender 4.3.2
Traceback (most recent call last):
  File "/tmp/kk-blender-abc/script.py", line 120, in <module>
  File "chat_code", line 2, in <module>
  File "/opt/blender/4.3/scripts/modules/bpy/ops.py", line 109, in __call__
TypeError: Converting py args to operator properties: : keyword "radius" unrecognized
'''
t = explain_error(log, code)
check("is a str", isinstance(t, str))
check("names the line number", "line 2" in t)
check("quotes the failing source line", "primitive_cube_add(radius=1)" in t)
check("quotes the error", 'keyword "radius" unrecognized' in t)
check("suggests the helpers", "add(" in t and "blender_scene" in t)
check("short", len(t) < 900)

t = explain_error('Traceback (most recent call last):\n  File "chat_code", line 1, in <module>\nNameError: name \'foo\' is not defined\n', "foo()")
check("NameError says which names exist", "not defined" in t and "add" in t and "color" in t)

log2 = "\n".join("noise line %d" % i for i in range(60)) + "\nSegmentation fault\n"
t = explain_error(log2, "x = 1")
check("no traceback: shows the end of the log", "Segmentation fault" in t and "noise line 0" not in t)
check("no traceback: still suggests blender_scene", "blender_scene" in t)

t = explain_error("Blender timed out after 300 s", "while True: pass")
check("timeout text kept", "timed out after 300 s" in t)
t = explain_error("", "")
check("empty log is fine", isinstance(t, str) and "blender_scene" in t)
print("ALL PASS" if not fails else f"{len(fails)} failed")
sys.exit(1 if fails else 0)
