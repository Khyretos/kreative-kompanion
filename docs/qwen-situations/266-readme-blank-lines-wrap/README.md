# 266: README blank lines and line wrap (SUN-01 unit sun-readme)

The Coder drafted `tools/host/sunshine/README.md` with every fact right, but with no blank line after three
headings and before the code fence, prose lines of 143-387 characters, and one prompt instruction copied
into the text ("shown in a ```lua block exactly as:").

- Run 1 (`wrong-1.md.txt`): the check reported only the first bad heading, so each of the two repair rounds
  fixed one heading and the third attempt still failed. That part was a check fault (Claude's): the check now
  lists every bad line (`check-readme.py`).
- Run 2 with the new check (`wrong-2.md.txt`): all headings fixed in the repair rounds, but no round wrapped
  the long lines; 3 attempts failed on "wrap ALL prose lines over 120 characters".
- Claude wrapped the lines, added the blank lines and removed the copied instruction (`corrected.md.txt`).

Rerun: `job.json` with `check-readme.py` as its check, from a tree that holds the three other
`tools/host/sunshine/` files. `trace.tgz` has every prompt and answer of both runs.
