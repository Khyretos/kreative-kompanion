-- OVR-01b: the Overseer answers every chat of this project (besides the per-chat switch).
ALTER TABLE projects ADD COLUMN overseer INTEGER NOT NULL DEFAULT 0;
