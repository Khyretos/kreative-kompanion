-- KG-04: the escalation ladder of a run: one entry per rung tried (model, why it ran, tokens, minutes,
-- fix rounds, how it ended), as JSON.
ALTER TABLE runs ADD COLUMN ladder TEXT NOT NULL DEFAULT '[]';
