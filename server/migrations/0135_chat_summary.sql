-- CTX-01: a running summary of the older part of a chat, so messages that no longer fit the prompt
-- are summarised instead of silently dropped. It covers every message up to (upto_at, upto_rowid),
-- in the order the prompt reads them (at, rowid); the newer messages go to the model word for word.
CREATE TABLE chat_summary (
    chat_id    TEXT PRIMARY KEY REFERENCES chats(id) ON DELETE CASCADE,
    text       TEXT NOT NULL,
    upto_at    TEXT NOT NULL,
    upto_rowid INTEGER NOT NULL,
    messages   INTEGER NOT NULL,
    at         TEXT NOT NULL
);
