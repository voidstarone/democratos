-- Platform-wide sensitive-content review cases. Deliberately NOT scoped to a
-- demos (unlike `reports`): sensitive/illegal content is reviewed by the
-- platform-wide reviewer pool, not a community jury. `is_open` is denormalized
-- from the JSONB status so the review-queue and badge-count queries stay cheap.
CREATE TABLE IF NOT EXISTS sensitive_cases (
    id      BIGINT  PRIMARY KEY,
    is_open BOOLEAN NOT NULL,
    data    JSONB   NOT NULL
);
CREATE INDEX IF NOT EXISTS sensitive_cases_open ON sensitive_cases (is_open);
