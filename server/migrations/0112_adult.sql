-- STU-01c: the adult-content right (questionable and explicit Studio ratings); set with
-- `kompanion-server user-adult <name> on|off`.
ALTER TABLE users ADD COLUMN adult INTEGER NOT NULL DEFAULT 0;
