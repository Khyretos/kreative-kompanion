-- KG-05: MCP tools inside task runs need a grant, like a computer's runner tools: a task step may call
-- a tool of a configured [[mcp]] server only while the user's grant for that server (all tools '*', or
-- one tool) has not expired.
CREATE TABLE mcp_grants (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server     TEXT NOT NULL,
    tool       TEXT NOT NULL DEFAULT '*',
    granted_at TEXT NOT NULL,
    expires    TEXT NOT NULL
);
CREATE INDEX mcp_grants_user ON mcp_grants (user_id);
