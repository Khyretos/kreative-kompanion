#!/usr/bin/env python3
"""chk.py FILE NEEDLE... : prints error: FILE: missing NEEDLE for each needle not in FILE (\\n = newline).
Whitespace runs and rustfmt's trailing commas (and a needle's own) are ignored, so wrapped signatures still match."""
import re, sys
# STU-02b: whitespace before a closing bracket goes too (a signature wrapped one parameter per line).
def norm(s): return re.sub(r",?\s*([)\]}])", r"\1", re.sub(r"([(\[{])\s+", r"\1", re.sub(r"\s+", " ", s)))
f=sys.argv[1]; t=norm(open(f).read()); bad=0
for n in sys.argv[2:]:
    # STU-01c: a needle's own trailing comma is dropped too (the last field before "}" has none; lesson 74).
    if norm(n.replace("\\n","\n")).rstrip(",") not in t: print(f"error: {f}: missing {n!r}"); bad=1
sys.exit(bad)
