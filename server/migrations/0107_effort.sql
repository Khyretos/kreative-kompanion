-- EF-01: how hard the models work. Levels: auto, low, medium, high.
ALTER TABLE chats ADD COLUMN effort TEXT NOT NULL DEFAULT 'auto';
ALTER TABLE tasks ADD COLUMN effort TEXT NOT NULL DEFAULT 'auto';
-- The level a model call ran at (null for calls before EF-01).
ALTER TABLE calls ADD COLUMN effort TEXT;
