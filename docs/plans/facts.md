# FACTS: per-project and per-chat facts, plus readable chat and command history

Spec: this chat (Loquendo, 2026-10-22, 02:26 to 02:39). Status: built on branch khyretos/facts-01 (2026-10-24, Claude-app parity work).

## Built, and where it differs from the plan below

- Migration is `0136_facts.sql` (0135 went to the context meter's chat summary).
- The model saves and changes its own facts without an approval card, like the Claude app's memory; it can never change or delete a fact you set (it is told to ask you). Every save shows as a tool use in the chat and is in the call log.
- `remember` / `forget` / `recall` are always there in computer chats; plain chats turn them on as "Remember facts" in the Tools menu (a tool round costs an extra model call per message there). The facts block is read into every prompt either way.
- `read_chat_history` is the `read_chat` tool of the Past chats PR (HIST-01), with `search_chats` beside it.
- `read_command_history` reads `pc_actions` with a cursor, results cut at 2,000 characters and masked; the 10-step digest is masked too. The one-off cleanup of stored rows is not done (it needs your OK).
- The only automatic check: a computer step that reports "no such file" for a path flags a path fact with that value (in this chat or its project). Nothing is ever deleted by a check.
- Facts is a tab in the right pane (next to Activity), opened from a bookmark button in the chat header; it shows the chat's project facts and its own, or a project's when no chat is open. Chats outside a project have chat facts only.
- The call log stores the fact ids of each chat answer in its request JSON (`facts`).

## What changed

Planned changes. Items marked (assumption) were not stated in the spec and need your OK.

### Facts store

- Migration `0135_facts.sql`: table `facts(id, project_id, chat_id, key, value, kind, source, created_at, updated_at, last_used_at, flagged_at)`.
  - A fact belongs to exactly one scope, enforced by a CHECK: a project fact has `project_id` set and `chat_id` NULL; a chat fact has `chat_id` set and `project_id` NULL. Both reference their table with ON DELETE CASCADE. Facts never cross projects, and moving a chat to another project cannot leave a stale link. The chat's project is found through `chats.project_id` when the prompt block is built.
  - One key per scope: two partial unique indexes, `(project_id, key)` and `(chat_id, key)`, because NULLs never collide in a plain UNIQUE.
  - A chat that is not in any project (`chats.project_id` is NULL) has only chat-level facts (assumption).
- Access follows project access: the owner and editors may add, edit and delete; viewers may only read (assumption, matches `project_members`).
- You can change every fact, whether you or the assistant created it (`source` is `user` or `assistant`). The assistant cannot change or delete a fact with `source = user` unless you approve that specific tool call. Editing an assistant fact switches its `source` to `user`, so the assistant does not silently overwrite your change.
- A fact can be anything: free text, a file or folder path, a URL, a place, a command, a note. The value is stored exactly as written and is not limited to file locations. `kind` (`text`, `path`, `url`, `location`, `other`) is only a label for display and optional checks. It never restricts what a value can hold. The assistant picks the kind, and you can change it when you edit.
- The one restriction is secrets: a fact may not hold a token or password itself. The server rejects values that look like tokens and returns an error saying so. A fact can still say where a secret lives, in any form ("Forgejo token: file ~/.forgejo-token", "in the password manager under Forgejo").

### Reading facts into prompts

- Each chat or task prompt gets one short facts block: project facts first, then chat facts. Each fact shows its age.
- The block has a size budget like skills have. Over budget, the least recently used facts drop out of the prompt (they are not deleted).
- The call log records which fact ids went into each call.

### Writing facts (tools)

- `remember(scope, key, value)` where scope is `project` or `chat`, and `forget(key, scope)`. Each call is approved by you like any other tool call. `recall(query)` looks up facts of the chat and its project by words in their name or value, also the ones left out of the prompt (no query: all, at most 30); it only reads.
- Only facts that can be checked get a check, such as a `path` fact whose path no longer exists. Free text and other kinds are never checked and never flagged automatically. A fact whose check fails gets `flagged_at` set and is shown as flagged, so you can update it when something moves or changes. It is never deleted silently.

### Facts page in the app (easy to find, add and remove)

- A "Facts" tab in the project panel and a "Facts" button in the chat header, both showing the same list for their scope.
- Chat view shows project facts and chat facts in two labeled groups, so you can see where each one comes from.
- Add: a key field and a value field with one Add button. Every fact, including ones the assistant added, has an Edit control that changes the key and value in place, and a Delete control. Delete shows an Undo toast of about 6 s, like Archive.
- Each fact shows who set it (you or the assistant) and when it was last changed.
- Search box over keys and values. Flagged facts get a chip and sort first.
- 44 px touch targets at phone width, light and dark themes.

### Chat history and command history reads

- `read_chat_history(chat_id?, before?, limit?)`: reads past messages of the current chat from `messages`, newest first, paged. Another chat is only readable if it is in the same project (assumption).
- `read_command_history(chat_id?, before?, limit?)`: reads the tool steps run in the chat (tool, arguments, state, exit code, start of output), separate from the messages.
  - Steps are already stored in `pc_actions` (migration `0014`: tool JSON, summary, state, result, created_at, decided_at, scoped by `user_id` and `chat_id`). No new table is needed, and `read_command_history` reads from it. `pcagent/steplog.rs` only gives a 10-step digest with results cut at 200 characters. The new tool reads further back and returns more of each result, still capped.
  - `pc_actions.result` and `tool` are stored unmasked today. The plan masks token-looking strings when the tool reads them and when the digest is built. A one-off cleanup of rows already stored, such as the Forgejo token pasted into this chat, is a separate step that needs your OK.
- Both tools are read-only. Results are capped in size and say when they were cut, so they never look complete when they are not.

## Tests

Planned, none run yet.

- cargo test: facts round trip per project; chat facts stay out of other chats; project facts stay out of other projects; viewer cannot write; token-looking value rejected; free text, path, URL and location values all stored unchanged; changing `kind` never changes the value; you can edit an assistant fact and it becomes yours; the assistant cannot change a user fact without approval; prompt block respects the budget and records fact ids in the call log; broken path fact is flagged and kept; free text is never flagged; `read_chat_history` paging and cap; `read_command_history` order, masking and cap.
- tsc clean; Playwright `tests/facts.spec.ts`: add, edit, delete with Undo and no reload; project and chat groups; search; flagged chip; 390 px light and dark with 44 px buttons; no sideways scroll.
- Full suite stays green.

## Open questions

1. Which file holds the Forgejo token? It gets recorded as a pointer once the feature exists.
2. Should chats outside any project have facts at all (assumption: chat-level only)?
3. Answered: the tool step log is stored in `pc_actions`, so `0136_tool_steps.sql` is not needed. The only migration is `0135_facts.sql`.
