-- M6-05: one row per workflow run, with its provenance (parameters, seed, models, licences).
CREATE TABLE studio_run (
    id TEXT PRIMARY KEY,
    workflow TEXT NOT NULL,
    gpu TEXT NOT NULL,
    user_id TEXT NOT NULL,
    params TEXT NOT NULL,       -- JSON: the filled parameters, seed included
    models TEXT NOT NULL,       -- JSON: [{file, licence, attribution}]
    state TEXT NOT NULL,        -- running, done, failed
    error TEXT,
    outputs TEXT NOT NULL DEFAULT '[]', -- JSON: file paths under [studio] output_dir
    started_at TEXT NOT NULL,
    ended_at TEXT
);
CREATE INDEX studio_run_workflow ON studio_run (workflow, started_at);
