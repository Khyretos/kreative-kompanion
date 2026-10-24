-- PROV-UI: model providers an admin adds in Settings, next to the [[provider]] entries of
-- kompanion.toml. The key is never stored: only the environment variable or the secret file
-- that holds it.
CREATE TABLE custom_providers (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    kind           TEXT NOT NULL CHECK (kind IN ('openai-compatible', 'anthropic')),
    base_url       TEXT NOT NULL,
    local          INTEGER NOT NULL DEFAULT 0,
    api_key_env    TEXT,
    api_key_file   TEXT,
    max_tokens     INTEGER,
    context_window INTEGER,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);
