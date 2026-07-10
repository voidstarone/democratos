-- Durable replay-nonce log for forwarded commands.
--
-- The command endpoint on a community's owner refuses a signed command whose
-- (node, nonce) it has already applied. Keeping that record only in process
-- memory meant a captured command could be replayed against a freshly-restarted
-- owner within the freshness window (2 x MAX_COMMAND_SKEW_SECS). Persisting it
-- here closes that window across restarts. Rows are prunable once past their
-- expiry — a nonce too old to pass the freshness check need not be retained.
CREATE TABLE IF NOT EXISTS command_nonces (
    node       BIGINT NOT NULL,           -- forwarding node id
    nonce      TEXT   NOT NULL,           -- the command's unique nonce
    expires_at BIGINT NOT NULL,           -- epoch secs after which it may be pruned
    PRIMARY KEY (node, nonce)
);
CREATE INDEX IF NOT EXISTS command_nonces_expiry ON command_nonces (expires_at);
