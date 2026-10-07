-- ACC-01: renewals are logged as kind 'renewed' (old and new expiry in detail).
-- SQLite cannot change a CHECK constraint in place: rebuild the table.
CREATE TABLE access_log_new (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    machine_id TEXT NOT NULL,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    at         TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('granted', 'revoked', 'renewed', 'used', 'refused')),
    target     TEXT,
    detail     TEXT
);
INSERT INTO access_log_new (id, machine_id, user_id, at, kind, target, detail)
    SELECT id, machine_id, user_id, at, kind, target, detail FROM access_log;
DROP TABLE access_log;
ALTER TABLE access_log_new RENAME TO access_log;
CREATE INDEX access_log_user ON access_log (user_id, id);
