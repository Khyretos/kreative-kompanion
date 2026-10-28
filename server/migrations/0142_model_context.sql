-- CTX-02: what Kompanion knows about the context size of a model, per provider and model, in tokens.
--   manual:   set by an admin in Settings (wins over everything)
--   learned:  the limit a server named when it refused a prompt as too long (only ever lowered, cleared by Detect)
--   detected: what the server said when asked (its model list, llama.cpp /props, Ollama, LM Studio, Anthropic)
-- A provider's own context_window (kompanion.toml or Settings) and the known sizes of common cloud models come after these.
CREATE TABLE model_context (
    provider_id   TEXT NOT NULL,
    model_id      TEXT NOT NULL,
    manual        INTEGER,
    detected      INTEGER,
    detected_from TEXT,
    detected_at   INTEGER,
    learned       INTEGER,
    learned_at    INTEGER,
    PRIMARY KEY (provider_id, model_id)
);
