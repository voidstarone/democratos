//! Outbound notification delivery (today: the invite-approval email).

use async_trait::async_trait;

use crate::NotifyError;

/// Delivers operator-to-user messages out of band. The only message so far is the
/// invite-approval email carrying the one-time accept link, so the port names that
/// use-case directly rather than exposing a generic "send arbitrary mail" surface.
///
/// Adapters: an SMTP sender for production and a log sender for dev (which just
/// prints the link). The composition root picks one; the application depends only
/// on this trait.
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Tell `to_email` they've been approved and hand them `accept_url` — the
    /// one-time link that finishes sign-up.
    async fn notify_invite_approved(
        &self,
        to_email: &str,
        accept_url: &str,
    ) -> Result<(), NotifyError>;
}
