-- KG-02: plans the planner proposes (a goal broken into milestones and tasks, or a re-plan after a
-- task failed). Nothing enters the task tree until the user applies a proposal.
CREATE TABLE plan_proposals (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    kind TEXT NOT NULL DEFAULT 'plan',
    reason TEXT NOT NULL DEFAULT '',
    plan TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL
);
CREATE INDEX plan_proposals_project ON plan_proposals(project_id, state);
