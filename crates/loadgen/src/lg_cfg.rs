//! A small connection pool config — loadgen shares the test Postgres with the nodes.

use adapter_store_postgres::PgStoreConfig;

/// A small connection pool — loadgen shares the test Postgres with the nodes.
pub(crate) fn lg_cfg() -> PgStoreConfig {
    PgStoreConfig {
        max_connections: 4,
        ..Default::default()
    }
}
