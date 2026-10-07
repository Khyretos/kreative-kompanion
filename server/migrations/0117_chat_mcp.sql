-- CHAT-01: the MCP tool servers a chat may use (JSON list of [[mcp]] names); none by default.
ALTER TABLE chats ADD COLUMN mcp TEXT NOT NULL DEFAULT '[]';
