#!/usr/bin/env python3
"""Build the AI capability Artifact page from the newest docs/ai-capability/report-*.md (step 11).
Usage: ai_capability_page.py OUT.html   (publish OUT.html with the Artifact tool, same path = same URL)"""

import glob
import html
import re
import sys

CSS = """
:root{--bg:#f6f4fa;--fg:#1d1a26;--mut:#5b5670;--card:#fff;--line:#d9d4e6;--pri:#6b3fd4;--sec:#b5560f}
@media (prefers-color-scheme:dark){:root:not([data-theme="light"]){--bg:#15121d;--fg:#f4f2fa;--mut:#aaa4c0;--card:#1e1a2a;--line:#37314a;--pri:#a98bf2;--sec:#f09a50;color-scheme:dark}}
:root[data-theme="dark"]{--bg:#15121d;--fg:#f4f2fa;--mut:#aaa4c0;--card:#1e1a2a;--line:#37314a;--pri:#a98bf2;--sec:#f09a50;color-scheme:dark}
body{background:var(--bg);color:var(--fg);font:15px/1.55 system-ui,sans-serif;padding-inline:16px;padding-block:24px;max-width:1100px;margin:0 auto}
h1{font-size:1.6rem;margin:0 0 .3em;text-wrap:balance}h2{font-size:1.15rem;color:var(--pri);margin:1.8em 0 .5em}
p,li{max-width:70ch}.mut{color:var(--mut)}
.t{overflow-x:auto;border:1px solid var(--line);border-radius:8px;background:var(--card)}
table{border-collapse:collapse;width:100%;min-width:760px;font-size:13px}
th,td{text-align:left;vertical-align:top;padding:7px 10px;border-bottom:1px solid var(--line)}
th{color:var(--pri);font-weight:600;position:sticky;top:0;background:var(--card)}
td:first-child{white-space:nowrap;font-variant-numeric:tabular-nums}
"""


def inline(s):
    s = html.escape(s)
    return re.sub(
        r"\*\*(.+?)\*\*",
        r"<strong>\1</strong>",
        re.sub(r"`(.+?)`", r"<code>\1</code>", s),
    )


def main(out):
    path = sorted(glob.glob("docs/ai-capability/report-*.md"))[-1]
    lines = open(path, encoding="utf-8").read().splitlines()
    body, i = [], 0
    while i < len(lines):
        ln = lines[i]
        if ln.startswith("# "):
            body.append(f"<h1>{inline(ln[2:])}</h1>")
        elif ln.startswith("## "):
            body.append(f"<h2>{inline(ln[3:])}</h2>")
        elif ln.startswith("|"):
            rows = []
            while i < len(lines) and lines[i].startswith("|"):
                if not re.match(r"^\|[\s\-|]+\|$", lines[i]):
                    rows.append([c.strip() for c in lines[i].strip("|").split("|")])
                i += 1
            head = "".join(f"<th>{inline(c)}</th>" for c in rows[0])
            trs = "".join(
                "<tr>" + "".join(f"<td>{inline(c)}</td>" for c in r) + "</tr>"
                for r in rows[1:]
            )
            body.append(f'<div class="t"><table><tr>{head}</tr>{trs}</table></div>')
            continue
        elif ln.strip():
            body.append(f"<p>{inline(ln)}</p>")
        i += 1
    page = f"<title>Kompanion AI Capability</title><style>{CSS}</style>" + "\n".join(
        body
    )
    open(out, "w", encoding="utf-8").write(page)


if __name__ == "__main__":
    main(sys.argv[1])
