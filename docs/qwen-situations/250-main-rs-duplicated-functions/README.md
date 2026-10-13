# 250-main-rs-duplicated-functions

main.rs (324 lines). Three tiny jobs; the result duplicated `now()` and `civil()` (the REPLACE block of default_grants swallowed the next function), left `default_config` undefined, and made unrelated edits (`let mut grants` to `let grants`, `let _ = grants.revoke`). Same family as 245: on a file with neighbouring functions give the exact old and new text of every changed line and say `copy every other line unchanged`; check `git diff --stat` of the unit.
