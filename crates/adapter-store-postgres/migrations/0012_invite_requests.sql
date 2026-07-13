-- The node-local access waitlist and operator settings that gate account
-- creation while a node runs invitation-only.
--
-- Deliberately NOT federated: neither table has an outbox trigger, so invite
-- requests and the invite-only toggle live and die on the node that owns them.
-- The full request document rides in JSONB `data`; only the query keys (email,
-- token_hash, status) are promoted to typed columns.
CREATE TABLE IF NOT EXISTS invite_requests (
    id           BIGINT PRIMARY KEY,
    email        TEXT   NOT NULL UNIQUE,
    -- SHA-256 (hex) of the one-time token; NULL until approved. The raw token is
    -- never stored (a leaked table yields no working links).
    token_hash   TEXT,
    status       TEXT   NOT NULL,
    requested_at BIGINT NOT NULL,
    data         JSONB  NOT NULL
);
-- The accept-link lookup and the review-queue scan.
CREATE INDEX IF NOT EXISTS invite_requests_token_hash ON invite_requests (token_hash);
CREATE INDEX IF NOT EXISTS invite_requests_status ON invite_requests (status);

-- A tiny key/value corner for operator settings that must survive a restart
-- (today: the invitation-only toggle). Node-local like the waitlist above.
CREATE TABLE IF NOT EXISTS node_settings (
    key   TEXT  PRIMARY KEY,
    value JSONB NOT NULL
);
