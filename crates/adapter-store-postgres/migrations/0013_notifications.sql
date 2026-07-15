-- Node-local in-app notifications (mentions + jury summons). Deliberately NOT in
-- the outbox replication set (see 0002_outbox): notifications are per-node
-- presentation state, generated where the triggering content/trial is created,
-- never federated. `seen` is denormalized from the JSONB so the unread-count
-- badge query stays cheap.
CREATE TABLE IF NOT EXISTS notifications (
    id        BIGINT  PRIMARY KEY,
    recipient BIGINT  NOT NULL,
    seen      BOOLEAN NOT NULL,
    data      JSONB   NOT NULL
);
-- Newest-first list per recipient.
CREATE INDEX IF NOT EXISTS notifications_recipient ON notifications (recipient, id DESC);
-- Unread-count badge: only the unseen rows per recipient.
CREATE INDEX IF NOT EXISTS notifications_unseen ON notifications (recipient) WHERE NOT seen;
