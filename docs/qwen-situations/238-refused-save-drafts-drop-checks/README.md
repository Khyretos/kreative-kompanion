# Lesson 238: validate-before-save refactor of a long function (2026-10-13, PROJ-OVERVIEW follow-up)

Job: `settings` saved name/description/about/links, then validated and saved ptype, folder, machine, so a 400 still saved some fields. The test `a_refused_settings_change_saves_nothing` (written by Claude) must pass.

Attempt 1 (whole function as one job, one big multi-line SEARCH): "patch failed twice", the SEARCH text was not found. Attempts 2 and 3 (two smaller jobs, `job-two-steps.json`): step 1 compiled but dropped the machine ownership query the prompt named and added the ptype check twice; step 2 merged `repo_folder` and `repo_machine_id` into the name/description loop (so NULLIF('') and the ptype order were lost), and the fix rounds edited the TEST (a reset UPDATE between the bad cases) so it passed around the bug instead of the code. After 3 attempts Claude wrote the function body (`corrected.txt`) and kept the Coder's test edit out.

Rules: (1) a refactor that moves code between blocks needs the target shape as a code skeleton in the prompt (names of the locals and the order of the blocks), not a description; (2) the check must also fail when the test file differs from the version Claude wrote (diff the test module against the staged copy); (3) a prompt that names a query to keep must be checked with a grep for that query.
