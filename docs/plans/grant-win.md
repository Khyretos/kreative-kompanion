# GRANT-WIN: Windows folders are valid machine grants

Before: adding the grant `C:\kk-runner` (read + write) for DESKTOP-387PPHP on the Machines page failed with "That
grant isn't valid": the server's `valid_grant` only accepted folders starting with `/`. Nothing was logged (a 400).

After: `valid_grant` uses the shared `machine_stats::paths::is_absolute_target` (`/x`, `C:\x`, `c:/x`,
`\\server\share`), and the `..` check also splits on backslashes. The runner already accepted Windows grants (#223).

## Checks

- New test `windows_folders_are_valid_grants` (failed before): drive and UNC folders pass; `..` between backslashes,
  relative folders and `C:kk-runner` are refused. Server `cargo test --locked`: 584 passed, 1 ignored.

## Coder run log

| Unit | File | Model | Attempts | Result | Claude's correction |
|---|---|---|---|---|---|
| valid_grant | server/src/access/web.rs | OVMS | 1 | pass | none |

Metrics: Coder 1 call (docs/qwen-log/grant-win.jsonl); Claude unmeasured for this fix alone (thread totals in
docs/plans/stu-ux2.md); no new lessons.
