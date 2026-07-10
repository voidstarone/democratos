//! The replication side of the Postgres store: draining this node's outbox, and
//! applying a peer's verified changes to the local replica.
//!
//! This module deals only in *rows* — the signing and signature-verification of
//! events lives in the pure `federation` crate, and the wiring that carries them
//! between nodes lives in the composition root / an HTTP adapter. Keeping the
//! store federation-agnostic (it never imports `federation`) means the crypto has
//! exactly one home.
//!
//! # Injection safety
//!
//! An event's `entity` names a table, and applying it must build SQL that
//! mentions that table. Table names therefore come **only** from a fixed
//! allowlist ([`entity_spec`](entity_spec::entity_spec)) that maps an entity
//! string to a compile-time table literal and a fixed primary-key predicate —
//! never from the event payload. An unknown entity is rejected outright, so a
//! malicious event cannot smuggle SQL through the table name. All row values flow
//! through bound parameters.

pub mod apply_mode;
pub mod entity_spec;
pub mod incoming_change;
pub mod outbox_record;
pub mod store_ops;
