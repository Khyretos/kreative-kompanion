"""STU-R2: write a ComfyUI API graph like the shipped ones (indent 2, links on one line)."""

import json
import re
import sys
from pathlib import Path


def dump(g, f):
    s = json.dumps(g, indent=2, ensure_ascii=False)
    s = re.sub(r'\[\s+("[^"\n]*"|-?\d+(?:\.\d+)?),\s+(-?\d+)\s+\]', r"[\1, \2]", s)
    Path(f).write_text(s + "\n")


if __name__ == "__main__":
    for f in sys.argv[1:]:
        dump(json.loads(Path(f).read_text()), f)
