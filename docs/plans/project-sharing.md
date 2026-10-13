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


## Slice 2 (web), 2026-10-13

Server: `GET /projects/{id}/overview` now returns `role` and `members` (`userId`, `name`, `role`, `you`); `GET /chats` lists the chats of projects shared with the user (member's pins are not shown as pinned); `GET /chats/{id}/messages` is readable by members (`api::can_read_chat`); `kompanion-server import` accepts `about` and `links` per project (needed for the About drafts; links must be http/https, at most 10). Posting stays owner-only until slice 3.

Web: the sidebar shows a `shared by <owner>` chip; the Overview has a members row (owner chip, one chip per member with role, the owner adds by name and removes, a member can only leave); a non-owner gets a read-only UI (no edit, no Add task, no Settings, a disabled composer with a read-only note, no Unarchive); `?demo&shared` adds a project mara shared with you as a viewer.

Tests: `server/src/sharing_tests.rs` (6 tests), `server/src/import_tests.rs` (2), `web/tests/projshare.spec.ts` (7 at 1280 dark/light and 390 px). Full server suite 574 passed; 66 web tests passed locally (projshare + projects, projarchive, projoverview, sidebar, newchat, projecttypes, contrast, layout, back).

### Per-unit run log (Coder on OVMS, kireserver)
- Units drafted: 41; passed their check on the first attempt: 36; fix rounds drafted: 4, fix rounds cut off at the context limit: 4 (lesson 241).
- Claude corrected three units by hand (lessons 240, 242, 243): 4 lines restored in mock.ts, 7 lines + 1 character in overview.ts, 1 line removed in main.ts.
- GPU seconds 466, output tokens 11952, prompt tokens 307648 (docs/qwen-log/khyretos-sharing-web-ovms.jsonl).
- Production diff (server + web, tests excluded): +225 / -10 lines; all production lines were drafted by the Coder except the ~12 corrected lines above.
