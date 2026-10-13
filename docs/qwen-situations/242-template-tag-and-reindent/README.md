# Lesson 242: template tag and reindent (2026-10-13, PROJECT-SHARING slice 2, web/src/views/overview.ts)

Unit(s): s2-o-helper, s2-o-members

Two faults in one file. (a) The job that inserts `isOwner()` re-indented the whole neighbouring `section()` template (3 check attempts; the rule 'change nothing else' was ignored). (b) In `membersRow` the remove button was written `aria-label="..."${icon("close")}</button>` with the `>` of the start tag missing; tsc cannot see HTML inside a template string, so only the Playwright run could show it. Claude restored `section()` and added the `>`.

Rule: diff the whole file after each unit (lesson 234 again); for a function that returns HTML, spell the tag text out char for char and run the spec before the next unit; a check can count `<button` against `</button>` in the new function.

Files: `job.json` (the job prompt and focus), `wrong.diff` (the Coder's change against the base), `corrected.diff` (the final change). The input is the file at the base commit of the branch.
