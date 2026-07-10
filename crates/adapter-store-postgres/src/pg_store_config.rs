//! Connection-pool tuning for [`PostgresStore::connect_with`].

/// Connection-pool tuning for [`PostgresStore::connect_with`].
///
/// The defaults suit a single node; a busy production node should raise
/// `max_connections` toward its Postgres `max_connections` (minus headroom for
/// replication and admin) and may shorten `statement_timeout_ms` to shed slow
/// queries faster.
#[derive(Debug, Clone, Copy)]
pub struct PgStoreConfig {
    /// Maximum pooled connections. Default 16.
    pub max_connections: u32,
    /// Server-side `statement_timeout` applied to every connection, in
    /// milliseconds. A query running longer is aborted, so one pathological
    /// request cannot pin a connection indefinitely. `0` disables the timeout
    /// (Postgres' own default — not recommended). Default 30_000 (30s).
    pub statement_timeout_ms: u64,
}

impl Default for PgStoreConfig {
    fn default() -> Self {
        Self {
            max_connections: 16,
            statement_timeout_ms: 30_000,
        }
    }
}
