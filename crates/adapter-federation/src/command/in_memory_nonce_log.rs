use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::command::nonce_log::NonceLog;
use crate::ForwardError;

/// Process-local nonce log — for single-box/dev and tests. **Not durable across a
/// restart**: use the Postgres-backed log ([`NonceLog for PostgresStore`]) in a
/// real cluster so a captured command can't be replayed against a rebooted owner.
#[derive(Default)]
pub struct InMemoryNonceLog {
    seen: Mutex<HashMap<(u16, String), i64>>,
}

#[async_trait]
impl NonceLog for InMemoryNonceLog {
    async fn remember(
        &self,
        node: u16,
        nonce: &str,
        now: i64,
        expiry_at: i64,
    ) -> Result<bool, ForwardError> {
        let mut seen = self.seen.lock().expect("nonce log mutex");
        // Drop entries too old to still be replayable, so the map stays bounded.
        seen.retain(|_, expiry| *expiry > now);
        Ok(seen.insert((node, nonce.to_string()), expiry_at).is_none())
    }
}
