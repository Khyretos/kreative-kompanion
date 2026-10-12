-- KG-08: capability matrix. Per model and task type: how often it finished a task of that type on its own.
-- Fed by finished task runs and by benchmark results (POST /api/capabilities/bench); read by the task router.
CREATE TABLE capability (
    provider   TEXT NOT NULL,
    model      TEXT NOT NULL,
    task_type  TEXT NOT NULL,
    tried      INTEGER NOT NULL DEFAULT 0,
    passed     INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (provider, model, task_type)
);
