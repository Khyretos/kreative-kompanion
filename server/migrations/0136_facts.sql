-- FACTS: things worth remembering, per project or per chat. A fact is free text: a note, a path,
-- a URL, a place, a command. `kind` is only a display label and never limits the value.
-- A fact belongs to exactly one scope: a project (chat_id NULL) or a single chat (project_id NULL),
-- so moving a chat between projects cannot leave a stale link.
CREATE TABLE facts (
    id           TEXT PRIMARY KEY,
    project_id   TEXT REFERENCES projects(id) ON DELETE CASCADE,
    chat_id      TEXT REFERENCES chats(id) ON DELETE CASCADE,
    key          TEXT NOT NULL,
    value        TEXT NOT NULL,
    kind         TEXT NOT NULL DEFAULT 'text' CHECK (kind IN ('text', 'path', 'url', 'location', 'other')),
    source       TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'assistant')),
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    last_used_at TEXT,
    flagged_at   TEXT,
    CHECK ((project_id IS NULL) <> (chat_id IS NULL))
);

-- One key per scope. Two partial indexes because NULLs never collide in a plain UNIQUE.
CREATE UNIQUE INDEX facts_project_key ON facts (project_id, key) WHERE project_id IS NOT NULL;
CREATE UNIQUE INDEX facts_chat_key ON facts (chat_id, key) WHERE chat_id IS NOT NULL;
