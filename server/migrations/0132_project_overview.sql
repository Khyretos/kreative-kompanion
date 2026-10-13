-- PROJ-OVERVIEW: add project metadata columns and backfill creation timestamps
ALTER TABLE projects ADD COLUMN about TEXT NOT NULL DEFAULT '';
ALTER TABLE projects ADD COLUMN links TEXT NOT NULL DEFAULT '[]';
ALTER TABLE projects ADD COLUMN repo_git TEXT NOT NULL DEFAULT '';
ALTER TABLE projects ADD COLUMN created_at TEXT;

UPDATE projects
SET created_at = (
    COALESCE(
        (SELECT MIN(at) FROM (
            SELECT te.at FROM task_events te
            JOIN tasks t ON t.id = te.task_id
            WHERE t.project_id = projects.id
            UNION ALL
            SELECT m.at FROM messages m
            JOIN chats c ON c.id = m.chat_id
            WHERE c.project_id = projects.id
        )),
        NULL
    )
);

UPDATE projects
SET created_at = updated_at
WHERE created_at IS NULL;

CREATE TRIGGER projects_created_at
AFTER INSERT ON projects
WHEN NEW.created_at IS NULL
BEGIN
    UPDATE projects SET created_at = NEW.updated_at WHERE id = NEW.id;
END;
