use async_trait::async_trait;

use crate::ForwardError;

/// A backend that records seen command nonces. `remember` returns `true` if
/// `(node, nonce)` was newly recorded (admit the command) or `false` if it was
/// already present (a replay). Implementations may prune entries past `expiry_at`.
#[async_trait]
pub trait NonceLog: Send + Sync {
    async fn remember(
        &self,
        node: u16,
        nonce: &str,
        now: i64,
        expiry_at: i64,
    ) -> Result<bool, ForwardError>;
}

/// Durable, Postgres-backed nonce log — survives an owner restart (M8).
#[async_trait]
impl NonceLog for adapter_store_postgres::PostgresStore {
    async fn remember(
        &self,
        node: u16,
        nonce: &str,
        now: i64,
        expiry_at: i64,
    ) -> Result<bool, ForwardError> {
        self.remember_command_nonce(node as i64, nonce, now, expiry_at)
            .await
            .map_err(|e| ForwardError::OwnerUnreachable(e.to_string()))
    }
}
