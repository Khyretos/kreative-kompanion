#!/usr/bin/env python3
"""A fake Blender for test_server.py: runs nothing, reads the --python script it got."""
import re, sys, base64
args = sys.argv[1:]
assert "-b" in args and "--factory-startup" in args, args
script = open(args[args.index("--python") + 1]).read()
print("stub blender: script has", len(script), "chars")
if "RAISE" in script:
    print("Traceback (most recent call last):\nValueError: RAISE", file=sys.stderr)
    sys.exit(1)
if "SLEEP" in script:
    import time; time.sleep(30)
m = re.search(r"finish\((['\"])(.+?)\1", script)
if m:
    png = base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==")
    open(m.group(2), "wb").write(png)
    open(m.group(2).rsplit(".", 1)[0] + ".blend", "wb").write(b"BLENDER-v430")
    open(m.group(2).rsplit(".", 1)[0] + ".txt", "w").write("- box: 1.00 wide, 1.00 deep, 1.00 tall, centre (0.00, 0.00, 0.50); stands on the ground\nAll objects touch a neighbour; nothing floats.")
    print("rendered", m.group(2))
