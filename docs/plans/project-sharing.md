# PROJECT-SHARING (Kees said "Share", 2026-10-13)

Add people to a project with a role. Scope agreed in the thread: the owner adds another Kompanion user by name as viewer or editor. Viewers see the project, its tasks, chats and Overview read-only. Editors can also change tasks and chat. Only the owner archives, deletes, changes members or edits the computer access. The Overview shows a members row.

## Design

- `project_members (project_id, user_id, role viewer|editor, added_at)`, migration 0133. The owner stays `projects.user_id`.
- `sharing::access(db, user, project) -> Option<{owner_id, role}>`: one place that answers "may this user see this project, and as whom". Reading code runs with the owner's id (tasks, runs and machines are keyed by the owner), so a viewer sees exactly what the owner sees.
- Routes (owner only unless noted): `GET /projects/{id}/members` (any member), `PUT /projects/{id}/members` {name, role}, `DELETE /projects/{id}/members/{user}` (the owner, or a member removing themselves).
- Slice 1 (this thread): migration, access(), member routes, shared projects in `GET /projects` with `role` and `ownerName`, Overview and the task list (project filter) readable by members.
- Slice 2: web (sidebar chip "shared by <owner>", Overview members row with add/remove, read-only UI for viewers), chats of a shared project for members.
- Slice 3: editor writes (task changes, chat messages), the Overseer/planner/scheduler stay owner-only.

Done when (slice 1): the tests in `server/src/sharing_tests.rs` pass and a live check as the test user (owner) plus a second user shows the shared project and its overview.

## Slice 1 result (PR 207, main 2af3312, deployed 2026-10-13)

Live check as the test user (owner): GET /projects/{id}/members gives role owner and no members; adding an unknown name or yourself gives 400; the project list carries role and ownerName; the overview answers 200. A second user could not be used live (accounts exist only through OIDC); the member paths are covered by the 4 tests in server/src/sharing_tests.rs (570 server tests pass).

## Metrics

| What | Value |
|---|---|
| Coder (OVMS) | 5 units: migration, overview, tasks list, project list as drafted on the first try; sharing.rs (about 150 lines) failed 3 attempts on one line, Claude fixed it (lesson 239). 210 GPU s, 5,448 output / 77,941 prompt tokens |
| Lines | server +334 / -10 (incl. 4 tests and a 30-line migration/doc part); about 190 drafted lines kept, 1 line corrected |
| Review | PR-Agent 3 findings, all false positives (user names are UNIQUE), 1 ledger commit |
| Claude tokens | unmeasured (thread totals only) |
| Not done | web (members row, shared chip), chats of shared projects, editor writes: slices 2 and 3 |
