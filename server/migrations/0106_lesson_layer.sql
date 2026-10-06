-- SK-03 step 8: each lesson proposal says which layer it belongs to and who proposed it.
ALTER TABLE lessons ADD COLUMN layer TEXT NOT NULL DEFAULT 'private' CHECK (layer IN ('general', 'private'));
ALTER TABLE lessons ADD COLUMN proposed_by TEXT NOT NULL DEFAULT '';
