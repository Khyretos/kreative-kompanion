-- PROJECT-SHARING: people a project is shared with, as viewer or editor (the owner stays projects.user_id)
CREATE TABLE project_members (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor')),
    added_at TEXT NOT NULL,
    PRIMARY KEY (project_id, user_id)
);

CREATE INDEX project_members_user ON project_members (user_id);
