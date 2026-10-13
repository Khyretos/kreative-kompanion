# Lesson 240: insert before deleted neighbours (2026-10-13, PROJECT-SHARING slice 2, web/src/api/mock.ts)

Unit(s): s2-m-data

An 'insert this block right BEFORE the line `export class MockApi ...`' patch deleted the 4-line `const replies = [...]` array above that line (its SEARCH covered the neighbours, its REPLACE dropped them). The needle check passed (it only looks for the new text); tsc found `Cannot find name 'replies'` one job later, and the three fix rounds died on context (lesson 241). Claude put the array back (4 lines).

Rule: an insertion job gets a no-deletions check (`diff <(git show HEAD:FILE) FILE | grep -c '^<'` must be 0) as well as the needles; say 'every existing line stays'.

Files: `job.json` (the job prompt and focus), `wrong.diff` (the Coder's change against the base), `corrected.diff` (the final change). The input is the file at the base commit of the branch.
