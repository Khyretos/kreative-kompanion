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
