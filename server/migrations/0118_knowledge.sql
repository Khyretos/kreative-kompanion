-- CHAT-03: knowledge for chats: a user's collections of documents, split into chunks, searched with FTS5.
CREATE TABLE knowledge_collection (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    source     TEXT NOT NULL DEFAULT 'app',   -- app | openwebui
    created_at TEXT NOT NULL,
    UNIQUE (user_id, name)
);
CREATE TABLE knowledge_doc (
    id            INTEGER PRIMARY KEY,
    collection_id TEXT NOT NULL REFERENCES knowledge_collection(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    chars         INTEGER NOT NULL,
    added_at      TEXT NOT NULL
);
CREATE INDEX knowledge_doc_by_collection ON knowledge_doc(collection_id);
CREATE TABLE knowledge_chunk (
    id     INTEGER PRIMARY KEY,
    doc_id INTEGER NOT NULL REFERENCES knowledge_doc(id) ON DELETE CASCADE,
    n      INTEGER NOT NULL,
    text   TEXT NOT NULL
);
CREATE INDEX knowledge_chunk_by_doc ON knowledge_chunk(doc_id);
CREATE VIRTUAL TABLE knowledge_fts USING fts5(text, content = 'knowledge_chunk', content_rowid = 'id', tokenize = 'porter unicode61');
CREATE TRIGGER knowledge_chunk_ai AFTER INSERT ON knowledge_chunk BEGIN
    INSERT INTO knowledge_fts(rowid, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER knowledge_chunk_ad AFTER DELETE ON knowledge_chunk BEGIN
    INSERT INTO knowledge_fts(knowledge_fts, rowid, text) VALUES ('delete', old.id, old.text);
END;
