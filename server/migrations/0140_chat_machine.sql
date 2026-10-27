-- PC-SYNC: the computer picked for a chat, so every device that opens the chat shows the same one
-- ('' = none). Kept per chat on the server instead of per browser.
ALTER TABLE chats ADD COLUMN machine_id TEXT NOT NULL DEFAULT '';
