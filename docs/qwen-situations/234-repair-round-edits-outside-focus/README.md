# Lesson 234: repair rounds edit outside the job's function (2026-10-13, PROJ-OVERVIEW)

Patch-mode repair rounds changed code the job did not own: o11's rounds rewrote where_part (`fetch_optional(s.db)`); w3's rounds duplicated a method body in http.ts after my own check failed on another file's errors; w4's rounds duplicated setGpuMode in mock.ts; w26's rounds removed the renderMarkdown import; w14's rounds added `ov-folder ov-folder` to the header's class to satisfy a grep needle (`class=\"ov-folder\"`).

Claude restored each file from the right draft. Rule: after every unit diff the whole file against the copy before the job and refuse changes outside the focused function; a check must fail only on the job's own file (filter other files' errors out), and needles should match inside the function they check.
