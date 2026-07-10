//! PostgreSQL implementation of every `*Store` port — the shared-database store
//! that makes the app horizontally scalable.
//!
//! # Why this exists
//!
//! [`adapter-store-textfile`](../adapter_store_textfile/index.html) keeps the
//! whole dataset in one process's RAM and rewrites a single JSON file on every
//! mutation, so exactly one process may own the data. This adapter persists to
//! Postgres instead: many app replicas can share one database, and — under the
//! federation layer — each node runs its own Postgres as the source of truth for
//! the communities it hosts.
//!
//! # Representation
//!
//! Each entity is stored as a `data` JSONB column (a lossless mirror of the
//! serde-serialized domain struct) plus a few typed columns lifted out only for
//! the lookups and aggregates the ports require. The JSONB keeps the adapter
//! exactly faithful to the domain model — the deeply nested enums (`ProposalKind`,
//! `ReportTarget`, `JurySizing`, `VoteWeighting`, `PostKind`, `Flag`) round-trip
//! with zero hand-written column mapping.
//!
//! # IDs
//!
//! IDs are minted through [`domain::compose_id`] as `node<<48 | sequence`, so no
//! two nodes collide without coordination. The 64-bit value is stored in a
//! `BIGINT` by bit-reinterpretation (`u64 as i64`); the column is only ever used
//! for equality and joins, never magnitude ordering, so a node whose id sets the
//! high bit (stored as a negative `BIGINT`) is fine.
//!
//! # Build & migrations
//!
//! Only the runtime query API is used (no compile-time `query!` macros), so the
//! crate builds with **no live database and no `sqlx` CLI**. The schema ships as
//! an embedded migration ([`sqlx::migrate!`]) applied at [`PostgresStore::connect`].

mod import;
mod is_insecure_url;
mod pg_store_config;
mod postgres_store;
mod replication;

pub use import::import_counts::ImportCounts;
pub use import::import_data::ImportData;
pub use import::jury_ballot_row::JuryBallotRow;
pub use import::post_vote_row::PostVoteRow;
pub use import::vote_row::VoteRow;
pub use is_insecure_url::is_insecure_url;
pub use pg_store_config::PgStoreConfig;
pub use postgres_store::PostgresStore;
pub use replication::incoming_change::IncomingChange;
pub use replication::outbox_record::OutboxRecord;
