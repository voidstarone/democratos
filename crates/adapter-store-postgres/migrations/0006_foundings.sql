-- Founding petitions — a demos-in-waiting that still needs co-signers.
--
-- Unlike the other entities, a petition is not yet scoped to a community (it IS
-- the not-yet-born community), so its fields are small and fixed: they are kept
-- as plain typed columns rather than a `data` JSONB document. The ordered set of
-- co-signers lives in a child table keyed by (founding_id, user_id) with an
-- explicit `position` so sign order round-trips exactly. Deleting a petition
-- (once founded or abandoned) cascades to its sign-offs.

CREATE TABLE IF NOT EXISTS foundings (
    id         BIGINT PRIMARY KEY,
    slug       TEXT   NOT NULL UNIQUE,
    name       TEXT   NOT NULL,
    founder    BIGINT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS founding_sign_offs (
    founding_id BIGINT NOT NULL REFERENCES foundings (id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL,
    position    INT    NOT NULL,
    PRIMARY KEY (founding_id, user_id)
);
CREATE INDEX IF NOT EXISTS founding_sign_offs_founding ON founding_sign_offs (founding_id);
