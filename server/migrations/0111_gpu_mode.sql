-- GPU-01: per paired computer, what its GPU is for: 'studio' (apps may run), 'gaming' (apps
-- stopped, Ollama unloaded) or 'auto' (stop after 15 min idle, start on a studio job, gaming
-- while a game runs). apps_stopped: Kompanion stopped the studio apps there; studio_at: the
-- last studio job sent to it.
ALTER TABLE machines ADD COLUMN gpu_mode TEXT NOT NULL DEFAULT 'auto';
ALTER TABLE machines ADD COLUMN apps_stopped INTEGER NOT NULL DEFAULT 0;
ALTER TABLE machines ADD COLUMN studio_at TEXT;
