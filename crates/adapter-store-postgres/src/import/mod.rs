//! Backfill import: load a whole dataset into this node's Postgres **preserving
//! original IDs**, so a single-box `democratos.json` deployment can migrate to
//! the scalable store (and to a federation) without breaking a single reference.
//!
//! # Why not the normal `create` methods?
//!
//! Every `*Store::create` **mints a fresh ID** — correct for new entities, wrong
//! for a migration, where a proposal must keep the exact ID its votes point at.
//! This module inserts each row with its existing ID instead, in foreign-key
//! order, in one transaction, and idempotently (`ON CONFLICT DO NOTHING`) so a
//! re-run is a no-op rather than a duplicate-key error.
//!
//! # Keeping future IDs collision-free
//!
//! After loading, [`import`](PostgresStore::import) advances each per-kind ID
//! counter past the highest **local sequence** it imported for *this* node, so
//! the next `create` on this node cannot re-mint an imported ID. (IDs minted by
//! other origin nodes live in a different part of the key space and never
//! collide — see [`domain::compose_id`].)

pub mod import_counts;
pub mod import_data;
pub mod jury_ballot_row;
pub mod post_vote_row;
pub mod vote_row;
