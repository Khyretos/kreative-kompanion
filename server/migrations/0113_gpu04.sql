-- GPU-04: the game state lives in the database, so the CLI (studio-run, studio-audio) and the
-- server agree: gaming_until is now + the cooldown while a game runs and when it ends.
ALTER TABLE machines ADD COLUMN gaming_until TEXT;
-- The studio run a GPU job belongs to: a dropped job fails its run.
ALTER TABLE gpu_job ADD COLUMN run_id TEXT;
-- Runs left "running" by older versions never finish: mark them failed once.
UPDATE studio_run SET state = 'failed', error = 'stopped before it finished (found at an upgrade)',
    ended_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE state = 'running';
