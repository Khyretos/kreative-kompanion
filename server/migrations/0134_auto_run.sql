-- AUTO-01: run a computer's steps without asking first. Off by default, per chat (this also covers
-- overseer chats, which are chats) and per project (covers the project's thread and its chats).
-- There is no global switch. Tools without a grant on the computer are still refused.
ALTER TABLE chats ADD COLUMN auto_run INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN auto_run INTEGER NOT NULL DEFAULT 0;
-- How many tool steps one request may take (1-100, clamped by the server).
ALTER TABLE users ADD COLUMN max_steps INTEGER NOT NULL DEFAULT 10;
