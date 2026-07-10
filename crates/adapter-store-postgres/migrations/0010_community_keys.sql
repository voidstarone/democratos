-- A community's secret signing seed, held ONLY by its home node.
--
-- The home node uses this Ed25519 seed to sign (and later re-sign, to migrate) the
-- community's founder home binding — the per-community authority that makes "the
-- founder chose THIS host" enforceable. It is deliberately NOT in the outbox
-- capture set and NOT in the replication apply allowlist, so a community's signing
-- authority never leaves its home node: no peer can read it from the change feed
-- or write it via a replicated event.
CREATE TABLE IF NOT EXISTS community_keys (
    demos BIGINT PRIMARY KEY,
    seed  TEXT   NOT NULL   -- 32-byte hex Ed25519 seed (SECRET — home node only)
);
