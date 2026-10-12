-- KG-07: the project state summary, rewritten after every finished or merged task and fed to the planner
-- and the Overseer. Plain text, bounded (see statesum.rs); empty until the first rewrite.
ALTER TABLE projects ADD COLUMN state_summary TEXT NOT NULL DEFAULT '';
ALTER TABLE projects ADD COLUMN state_summary_at TEXT NOT NULL DEFAULT '';
