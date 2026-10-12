-- PROJ-ARCHIVE: add timestamp for when a project was archived (NULL = active)
ALTER TABLE projects ADD COLUMN archived_at TEXT;
