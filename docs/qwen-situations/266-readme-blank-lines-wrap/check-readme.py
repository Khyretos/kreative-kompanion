#!/usr/bin/env python3
import os, re, sys
F = "tools/host/sunshine/README.md"
def err(m):
    print(f"error: {F}: {m}"); sys.exit(1)
if not os.path.exists(F):
    err("not written")
t = open(F).read()
need = {
    "a '# ' title line first": t.startswith("# "),
    "the three file names": all(n in t for n in ("sun-01.conf", "kk-sunshine-session.sh", "install.sh")),
    "the command sh tools/host/sunshine/install.sh": "sh tools/host/sunshine/install.sh" in t,
    "the hook line hl.exec_cmd(\"sleep 3 && $HOME/.local/bin/kk-sunshine-session.sh\") in a code block": 'hl.exec_cmd("sleep 3 && $HOME/.local/bin/kk-sunshine-session.sh")' in t,
    "the line it replaces: hl.exec_cmd(\"sleep 3 && sunshine\")": 'hl.exec_cmd("sleep 3 && sunshine")' in t,
    "the check command journalctl --user -u app-dev.lizardbyte.app.Sunshine.service": "journalctl --user -u app-dev.lizardbyte.app.Sunshine.service" in t,
    "the undo (the .bak-sun01 file)": ".bak-sun01" in t,
    "the reason: 1,572 restarts on 11 Oct": "1,572" in t,
}
for k, ok in need.items():
    if not ok:
        err(f"missing {k}")
L = t.splitlines()
bad = [f"line {i+1} ({l!r})" for i, l in enumerate(L) if l.startswith("#") and i + 1 < len(L) and L[i + 1] != ""]
bad += [f"line {i+1}: put a blank line before the ``` fence" for i, l in enumerate(L) if l.startswith("```") and i and L[i - 1] != "" and not L[i - 1].startswith("```") and sum(x.startswith("```") for x in L[:i]) % 2 == 0]
if bad:
    err("every heading needs a blank line after it (markdownlint MD022), fix ALL of: " + "; ".join(bad))
inside, longl = False, []
for i, l in enumerate(L):
    if l.startswith("```"):
        inside = not inside
    elif not inside and len(l) > 120:
        longl.append(f"line {i+1} ({len(l)} chars)")
if longl:
    err("wrap ALL prose lines over 120 characters: " + "; ".join(longl))
if not t.endswith("\n") or t.endswith("\n\n"):
    err("must end with exactly one newline")
print("ok")
