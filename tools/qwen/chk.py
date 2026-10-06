#!/usr/bin/env python3
"""chk.py FILE NEEDLE... : prints error: FILE: missing NEEDLE for each needle not in FILE (\\n = newline).
Whitespace runs and rustfmt's trailing commas are ignored, so wrapped signatures still match."""
import re, sys
def norm(s): return re.sub(r",\s*([)\]}])", r"\1", re.sub(r"([(\[{])\s+", r"\1", re.sub(r"\s+", " ", s)))
f=sys.argv[1]; t=norm(open(f).read()); bad=0
for n in sys.argv[2:]:
    if norm(n.replace("\\n","\n")) not in t: print(f"error: {f}: missing {n!r}"); bad=1
sys.exit(bad)
