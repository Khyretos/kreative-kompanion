import json, re, sys
def dump(g, f):
    s = json.dumps(g, indent=2, ensure_ascii=False)
    s = re.sub(r'\[\s+("[^"\n]*"|-?\d+(?:\.\d+)?),\s+(-?\d+)\s+\]', r'[\1, \2]', s)
    open(f, "w").write(s + "\n")
if __name__ == "__main__":
    for f in sys.argv[1:]: dump(json.load(open(f)), f)
