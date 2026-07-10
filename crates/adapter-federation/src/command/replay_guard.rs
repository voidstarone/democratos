use crate::command::in_memory_nonce_log::InMemoryNonceLog;
use crate::command::max_command_skew_secs::MAX_COMMAND_SKEW_SECS;
use crate::command::nonce_log::NonceLog;
use crate::ForwardError;

/// Refuses replays of a signed command: the same `(node, nonce)` can't be applied
/// twice. Backed by a [`NonceLog`] — Postgres in production (durable across
/// restart), in-memory for dev/tests. Entries expire after
/// `2 × `[`MAX_COMMAND_SKEW_SECS`]: once a command is too old to pass the freshness
/// check, its nonce need not be retained.
pub struct ReplayGuard {
    log: std::sync::Arc<dyn NonceLog>,
}

impl ReplayGuard {
    /// A durable guard backed by `log` (pass an `Arc<PostgresStore>` in production).
    pub fn new(log: std::sync::Arc<dyn NonceLog>) -> Self {
        Self { log }
    }

    /// A process-local guard (dev/tests) — not durable across a restart.
    pub fn in_memory() -> Self {
        Self {
            log: std::sync::Arc::new(InMemoryNonceLog::default()),
        }
    }

    /// Admit a command identified by `(node, nonce)` issued at `issued_at`, judged
    /// against the owner's clock `now`. Returns `Err` if the command is outside the
    /// freshness window or its nonce was already seen (a replay). On success the
    /// nonce is recorded so a later resubmission is refused.
    pub async fn admit(
        &self,
        node: u16,
        nonce: &str,
        issued_at: i64,
        now: i64,
    ) -> Result<(), ForwardError> {
        if (now - issued_at).abs() > MAX_COMMAND_SKEW_SECS {
            return Err(ForwardError::Rejected(
                "stale or future-dated command (outside the freshness window)".into(),
            ));
        }
        let expiry_at = now + 2 * MAX_COMMAND_SKEW_SECS;
        if self.log.remember(node, nonce, now, expiry_at).await? {
            Ok(())
        } else {
            Err(ForwardError::Rejected(
                "duplicate command (already applied — replay refused)".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replay_guard_refuses_a_repeated_nonce_and_stale_commands() {
        let guard = ReplayGuard::in_memory();
        // First submission of a fresh command is admitted.
        assert!(guard.admit(5, "nonce-1", 1_000, 1_000).await.is_ok());
        // The same (node, nonce) again — a replay — is refused.
        assert!(matches!(
            guard.admit(5, "nonce-1", 1_000, 1_001).await,
            Err(ForwardError::Rejected(_))
        ));
        // A different node reusing the nonce value is fine (keyed by node too).
        assert!(guard.admit(6, "nonce-1", 1_000, 1_001).await.is_ok());
        // A command dated outside the freshness window is refused regardless.
        assert!(matches!(
            guard.admit(5, "nonce-2", 1_000, 1_000 + MAX_COMMAND_SKEW_SECS + 1).await,
            Err(ForwardError::Rejected(_))
        ));
        assert!(matches!(
            guard.admit(5, "nonce-3", 1_000 + MAX_COMMAND_SKEW_SECS + 1, 1_000).await,
            Err(ForwardError::Rejected(_))
        ));
    }
}
