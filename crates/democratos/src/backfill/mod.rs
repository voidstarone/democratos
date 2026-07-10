//! Read a single-box `democratos.json` snapshot and load it into a node's
//! Postgres, preserving IDs (see [`adapter_store_postgres::PostgresStore::import`]).
//!
//! The on-disk shape is the text-file store's persisted format. Rather than
//! couple to that adapter's private types, this module deserializes the stable
//! JSON directly into the importer's [`adapter_store_postgres::ImportData`].
//! Unknown fields (the text store's `next_*` counters) are simply ignored.

pub mod jury_ballot_rec;
pub mod load;
pub mod one;
pub mod post_vote_rec;
pub mod snapshot;
pub mod vote_rec;
