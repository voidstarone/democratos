//! A notifier that logs instead of sending — the dev / no-SMTP fallback.

use app::{Notifier, NotifyError};
use async_trait::async_trait;

/// Writes the invite-approval message to the server console rather than emailing
/// it. Use on a dev box, or on any node where SMTP is not configured: the operator
/// reads the one-time accept link off the log and passes it on by hand. Never
/// fails.
///
/// Prints to **stderr** to match the composition root's other operator messages
/// (which use `eprintln!`, not `tracing`) — this build installs no tracing
/// subscriber, so a `tracing::info!` here would vanish and the operator would
/// never see the link.
#[derive(Default)]
pub struct LogNotifier;

impl LogNotifier {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Notifier for LogNotifier {
    async fn notify_invite_approved(
        &self,
        to_email: &str,
        accept_url: &str,
    ) -> Result<(), NotifyError> {
        // The link is a one-time secret, so this is only appropriate for a
        // dev/self-hosted console the operator controls.
        eprintln!(
            "invite approved for {to_email} — no email sent (log notifier). \
             Send this one-time link to the invitee:\n    {accept_url}"
        );
        Ok(())
    }
}
