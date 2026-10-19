---
name: worker/python
description: Python scripts and services (ingest jobs, MCP servers, small web servers).
roles: [worker, reviewer]
tags: [python, psycopg, sql, scripts]
paths: ["**/*.py"]
---
# Worker: Python

1. (2026-10-04) psycopg 3: `conn.execute(sql, params).fetchall()`; `Connection.fetchall()` does not exist.
2. (2026-10-04) Never build SQL with f-strings around values or filters; pass parameters.
3. (2026-10-04) A `while ...: ... break` loop that splits text must keep the last piece; check the
   output has all the input (count characters or lines).
4. (2026-10-04) Keep code blocks whole when chunking markdown; never drop lines you cannot parse.
5. (2026-10-04) A 300-line script in one draft comes back half-done ("I will assume..." and `pass`):
   2-4 functions with exact signatures per draft.
6. (2026-10-06) Do what the prompt says word for word: "match os.path.basename(out)" means the
   base name, not the full path; "add to the module docstring" means the one at the top of the file,
   not the function's. Every listed change is part of the answer (a comment line too).
7. (2026-10-06) Use the paths the prompt names exactly (`"skills/" + key`), never a folder made up
   from the script's own location. A list in the input (`"lessons": [...]`) means every item: loop
   over it, never take `[0]`. A front-matter header ends at its second `---` line:
   `end = text.index("\n---\n", 4) + 5`. Pass the value the function reads (`data[core]`, a dict),
   not `list(data[core])` when it indexes by key.
8. (2026-10-06) Merging layers of a dict: one pass over the layers in order, never `while changed:`
   (a step that always rewrites an entry never settles and the tests hang). A later entry with the
   same key replaces the whole earlier entry, body included; an override takes the NEW body, an
   extend appends it. Give tests a `timeout 60` so a hang fails fast.
9. (2026-10-06) A function that returns a tuple is unpacked at every call (`_, blocks =
   split_card(text)`), and a file is read once, before any loop over its lines, not once per item.
10. (2026-10-06) Only inputs must exist: create output folders with `os.makedirs(parent,
    exist_ok=True)` instead of stopping with "directory not found".
163. (2026-10-11) A failing test is fixed in the code it describes: never change a function's return shape or
    load the wrong file just to make an assert pass. Cache exactly what the spec names (loaded models), not results
    keyed by an input path (a server sees a new temp path every request). Ultralytics boxes: `r.boxes.xyxy.tolist()`
    and `r.boxes.conf.tolist()`; `float()` of a 4-number tensor fails.
306. (2026-10-19, STU-SHEET3D) A numeric rule given in prose (subtract, rotate, flatten, compare) comes back
   reordered or inverted: copy such a rule as the literal code lines the prompt gives. A helper that returns a
   tuple returns something you cannot assign into: unpack it (`x, _, z = rotate(...)`), never `t[0] += ...`.
308. (2026-10-19, STU-SHEET3D) A CLI with several commands: read each command's arguments inside its own branch,
   at the positions the prompt lists for that command; write no file the prompt does not name.
343. (2026-10-19, reencoder) A function named like a question (is_/has_/<thing>_encode) only reads; migrations go
   where the connection opens; never re-import a module inside a function that already uses it; when one loop is
   split into two threads, each loop keeps only its own job.
