//! Which storage adapter the composition root wires.

use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum StoreKind {
    /// Ephemeral, in-process (lost on exit).
    Memory,
    /// Persisted to a single JSON text file. Single-writer, single-box.
    File,
    /// Shared PostgreSQL database. The scalable store: many app replicas can
    /// share one database, and each federated node runs its own as the source of
    /// truth for the communities it hosts. Requires `--database-url`.
    Postgres,
}
