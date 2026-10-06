-- EF-01 part 4: a default effort per role (auto = the task's level, else Auto picks), and whether
-- a run's effort was picked by Auto.
ALTER TABLE user_roles ADD COLUMN effort TEXT NOT NULL DEFAULT 'auto';
ALTER TABLE runs ADD COLUMN effort_picked INTEGER NOT NULL DEFAULT 0;
