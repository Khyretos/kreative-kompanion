# Lesson 221: the check's test filter also ran the next unit's test

Situation: PROJ-ARCHIVE unit s5a (server/src/overseer.rs, fn scan, OVMS). Job: s5a.json. The check ran `cargo test overseer::tests`, which also runs `a_task_proposed_by_name_never_reaches_an_archived_project`, the test for the NEXT unit (s5b). The checks had been validated on the untouched tree (fail) and on the full reference edit (pass), but never on the staged tree "everything up to s5a".

What happened: the Coder's first draft was right (one `AND archived_at IS NULL`), the check failed on the s5b test, and the two repair rounds, told to fix an error that was not in its code, made a whitespace-only change elsewhere in the file (s5a-coder-overseer.diff.gz). Claude reverted the whitespace and counted the unit as passed on the first draft.

Rule: a unit's check runs exactly that unit's test (`cargo test module::tests::name`). Validate each check on the staged tree before AND after its own unit, not only on base and the full reference (queue thread 8 said this; this thread skipped it).
