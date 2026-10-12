-- KC-01: Anthropic prompt caching. Tokens a call wrote to / read from the provider's prompt cache
-- (NULL: the provider reports none). They are not part of tokens_in.
ALTER TABLE calls ADD COLUMN cache_write INTEGER;
ALTER TABLE calls ADD COLUMN cache_read INTEGER;
