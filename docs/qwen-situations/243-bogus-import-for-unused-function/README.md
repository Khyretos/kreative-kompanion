# Lesson 243: bogus import for unused function (2026-10-13, PROJECT-SHARING slice 2, web/src/main.ts)

Unit(s): s2-a-composerfn

Claude's check fault first: the unit adds `applyComposerAccess()` and the call is the next unit, so `tsc` (noUnusedLocals) failed TS6133 on a correct function. Both fix rounds then 'fixed' it by inventing `import { applyComposerAccess } from "./views/conversation"` (TS2305: not exported there). Claude deleted the import line.

Rule: a function and its only call go into one job, or the check ignores TS6133 for that name. A repeated error that names a symbol as unused means 'add the caller', never an import.

Files: `job.json` (the job prompt and focus), `wrong.diff` (the Coder's change against the base), `corrected.diff` (the final change). The input is the file at the base commit of the branch.
