-- KG-03: the task queue scheduler. A project's scheduler settings are one JSON text
-- (enabled, machines with their folders, default check command, default budget); a task can
-- carry its own budget (NULL: the project's, then the built-in defaults).
ALTER TABLE projects ADD COLUMN scheduler TEXT NOT NULL DEFAULT '';
ALTER TABLE tasks ADD COLUMN budget_minutes INTEGER;
ALTER TABLE tasks ADD COLUMN budget_rounds INTEGER;
ALTER TABLE tasks ADD COLUMN budget_tokens INTEGER;
