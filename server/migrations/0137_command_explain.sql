-- CMD-01: "Explain running actions" on step cards: the user's settings (on, compact or verbose,
-- an optional model) and a cache of explanations, one per command, mode, language and model.
ALTER TABLE users ADD COLUMN explain_prefs TEXT NOT NULL DEFAULT '{}';
CREATE TABLE command_explanations (
    hash       TEXT PRIMARY KEY, -- sha256 of mode, language, provider/model and the command
    text       TEXT NOT NULL,    -- markdown
    model      TEXT NOT NULL,
    created_at TEXT NOT NULL
);
