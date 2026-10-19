# SHARED-LIVE: shared-project members see changes without a reload

Before: in a shared project, an editor who asked the orchestrator something saw their own message at once, but the
answer only after a reload. `send` hands the reply to `answer(..., owner_id, ...)`, and every live event went to that
one user (the owner). Task, chat and project list changes by the owner or by the scheduler also reached only that user,
so members had to reload those lists too.

After: chat questions and answers go to everyone who may read the chat (`sharing::chat_audience`: the chat's owner plus
the project's owner and members). A member's live stream also passes list changes ("tasks", "projects", "chats",
"project-assets") from anyone who shares a project with them (`sharing::collaborators`, reloaded when their projects
change). Private events (settings, machines, access, lessons, notifications, chat text outside the project) stay with
their user.

Found from the carried-over queue note "editors see answers only after reload" and Kees's standing rule (2026-10-03):
"I don't want to feel like I have to refresh after every action".

## Checks (written first)

- `sharing_tests.rs`: `chat_audience_is_the_owner_plus_the_project_members`, `collaborators_share_a_project_either_way`,
  `an_editor_sees_the_question_and_answer_live_in_a_shared_chat` (fails on the old api.rs: the owner never got the
  question or the answer).
- `events_tests.rs`: `send_many_reaches_each_user_once`, `a_listener_passes_its_own_events_and_shared_list_changes`.

## Coder run log (OVMS on kireserver, one pipeline run per unit, up to 3 attempts)

| Unit       | File                  | Attempts | Result | Claude's correction                                             |
| ---------- | --------------------- | -------- | ------ | --------------------------------------------------------------- |
| u1-sharing | server/src/sharing.rs | 3        | fail   | restored the replaced `chat_editor`, removed a fake import, `query_scalar` (lesson 359) |
| u2-events  | server/src/events.rs  | 2        | pass   | none                                                            |
| u3-sse     | server/src/api.rs     | 1        | pass   | none                                                            |
| u4-answer  | server/src/api.rs     | 3        | fail   | 3 lines: closure captures and one missed send (lesson 360)      |

Every Coder call is in docs/qwen-log/shared-live.jsonl; situations 359 and 360 in docs/qwen-situations/.
