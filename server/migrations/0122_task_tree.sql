-- KG-01: a task tree. A goal holds milestones, a milestone holds tasks, a task can hold subtasks;
-- task_deps says which tasks must be done before another can start (same project only).
ALTER TABLE tasks ADD COLUMN parent_id TEXT REFERENCES tasks(id) ON DELETE SET NULL;
ALTER TABLE tasks ADD COLUMN kind TEXT NOT NULL DEFAULT 'task';
CREATE INDEX tasks_parent ON tasks(parent_id);
CREATE TABLE task_deps (
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    depends_on TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, depends_on),
    CHECK (task_id <> depends_on)
);
CREATE INDEX task_deps_on ON task_deps(depends_on);
