-- OVR-01: a chat can be an Overseer chat; each user may name the Overseer and allow it to add
-- context to running tasks (NULL = the server's [overseer] default).
ALTER TABLE chats ADD COLUMN overseer INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN overseer_name TEXT;
ALTER TABLE users ADD COLUMN overseer_interject INTEGER;
