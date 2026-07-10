-- Democratos Postgres schema.
--
-- Shape: every entity keeps its authoritative representation in a `data` JSONB
-- column (a lossless mirror of the serde-serialized domain struct, so nested
-- enums like ProposalKind / ReportTarget / VoteWeighting round-trip exactly),
-- alongside a handful of typed columns lifted out purely for the WHERE-clauses
-- and aggregates the store ports need. This keeps the adapter faithful to the
-- domain model while still letting Postgres index the hot lookups.
--
-- IDs are stored as BIGINT holding the raw 64 bits of the domain's composite
-- `node<<48 | sequence` id (reinterpreted from u64; values with the node's high
-- bit set are negative BIGINTs — that is fine, the column is used only for
-- equality/joins, never magnitude ordering). Timestamps are unix seconds.

-- Per-node, per-kind monotonic sequence counters. The adapter composes the
-- returned value with this node's id to form the global composite id, so no two
-- nodes ever mint the same id and no coordination is required.
CREATE TABLE IF NOT EXISTS id_counters (
    kind TEXT PRIMARY KEY,
    next BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id         BIGINT PRIMARY KEY,
    handle     TEXT   NOT NULL UNIQUE,
    created_at BIGINT NOT NULL,
    data       JSONB  NOT NULL
);

CREATE TABLE IF NOT EXISTS demoi (
    id         BIGINT PRIMARY KEY,
    slug       TEXT   NOT NULL UNIQUE,
    created_at BIGINT NOT NULL,
    data       JSONB  NOT NULL
);

CREATE TABLE IF NOT EXISTS memberships (
    user_id         BIGINT  NOT NULL,
    demos_id        BIGINT  NOT NULL,
    tier            TEXT    NOT NULL,
    enfranchised_at BIGINT,
    data            JSONB   NOT NULL,
    PRIMARY KEY (user_id, demos_id)
);
CREATE INDEX IF NOT EXISTS memberships_demos ON memberships (demos_id);
CREATE INDEX IF NOT EXISTS memberships_user  ON memberships (user_id);

CREATE TABLE IF NOT EXISTS proposals (
    id       BIGINT PRIMARY KEY,
    demos_id BIGINT NOT NULL,
    data     JSONB  NOT NULL
);
CREATE INDEX IF NOT EXISTS proposals_demos ON proposals (demos_id);

CREATE TABLE IF NOT EXISTS votes (
    proposal_id BIGINT  NOT NULL,
    voter_id    BIGINT  NOT NULL,
    aye         BOOLEAN NOT NULL,
    weight      BIGINT  NOT NULL,
    PRIMARY KEY (proposal_id, voter_id)
);

CREATE TABLE IF NOT EXISTS post_votes (
    post_id BIGINT  NOT NULL,
    user_id BIGINT  NOT NULL,
    up      BOOLEAN NOT NULL,
    PRIMARY KEY (post_id, user_id)
);
CREATE INDEX IF NOT EXISTS post_votes_user ON post_votes (user_id);

CREATE TABLE IF NOT EXISTS rules (
    id       BIGINT  PRIMARY KEY,
    demos_id BIGINT  NOT NULL,
    active   BOOLEAN NOT NULL,
    data     JSONB   NOT NULL
);
CREATE INDEX IF NOT EXISTS rules_demos ON rules (demos_id);

CREATE TABLE IF NOT EXISTS posts (
    id         BIGINT PRIMARY KEY,
    demos_id   BIGINT NOT NULL,
    author     BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    data       JSONB  NOT NULL
);
CREATE INDEX IF NOT EXISTS posts_demos  ON posts (demos_id);
CREATE INDEX IF NOT EXISTS posts_author ON posts (author);

CREATE TABLE IF NOT EXISTS comments (
    id         BIGINT PRIMARY KEY,
    post_id    BIGINT NOT NULL,
    author     BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    data       JSONB  NOT NULL
);
CREATE INDEX IF NOT EXISTS comments_post   ON comments (post_id);
CREATE INDEX IF NOT EXISTS comments_author ON comments (author);

CREATE TABLE IF NOT EXISTS reports (
    id       BIGINT  PRIMARY KEY,
    demos_id BIGINT  NOT NULL,
    is_open  BOOLEAN NOT NULL,
    data     JSONB   NOT NULL
);
CREATE INDEX IF NOT EXISTS reports_demos ON reports (demos_id);

CREATE TABLE IF NOT EXISTS trials (
    id       BIGINT PRIMARY KEY,
    demos_id BIGINT NOT NULL,
    verdict  TEXT   NOT NULL,
    data     JSONB  NOT NULL
);
CREATE INDEX IF NOT EXISTS trials_demos ON trials (demos_id);

CREATE TABLE IF NOT EXISTS jury_ballots (
    trial_id BIGINT  NOT NULL,
    juror_id BIGINT  NOT NULL,
    guilty   BOOLEAN NOT NULL,
    weight   BIGINT  NOT NULL,
    PRIMARY KEY (trial_id, juror_id)
);
