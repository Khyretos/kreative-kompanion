-- CHAT-09: the largest file one attachment may be, in MB, per user. 0 = no limit (the default: this is a local tool).
ALTER TABLE users ADD COLUMN max_upload_mb INTEGER NOT NULL DEFAULT 0;
