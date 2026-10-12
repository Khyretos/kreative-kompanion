# Lesson 222: the Coder adds things nobody asked for, and the extras break the build or duplicate entries

Situation 1: PROJ-ARCHIVE unit w2c, patch web/src/api/mock.ts (demo API setProjectSettings), OVMS, 3 attempts failed. Job: w2c.json; before: w2c-before.txt.gz. Asked: add `archived?: boolean` to the change type and one line that sets `p.archivedAt`.
What the Coder did (w2c-wrong.diff.gz): also added a new `archiveProject()` method to the mock, and pushed "archived"/"restored" entries into the `history` list, which holds machine access events. tsc refused the kinds; both repair rounds kept the extras.

Situation 2: unit w4d, patch web/src/views/sidebar.ts (Move to project submenu), OVMS, 3 attempts failed. Job: w4d.json; before: w4d-before.txt.gz. Asked: change `s.projects.map(` to `s.projects.filter((p) => s.showArchived || !p.archivedAt).map(`.
What the Coder did (w4d-wrong.diff.gz): the filter plus a `.sort(newest)` and a second "Archived" group that lists every archived project again, so with "Show archived" on they appeared twice.

Fix: Claude removed the extras (the corrected diffs), keeping the requested lines.

Rule (lesson 196 again): "Change nothing else" does not stop the 9B from adding related features. Checks should also bound the size of the change (lines added) or fail on new methods/blocks, and prompts for one-line edits should say "the answer is exactly one SEARCH/REPLACE block of one line".
