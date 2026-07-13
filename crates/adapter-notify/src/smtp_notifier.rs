//! The production notifier: sends the invite-approval email over SMTP.

use app::{Notifier, NotifyError};
use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::smtp_config::SmtpConfig;

/// Sends real email over authenticated, TLS-encrypted SMTP. Construct once at
/// boot ([`new`](Self::new) validates the server settings and the `From:`
/// address); each [`notify_invite_approved`](Notifier::notify_invite_approved)
/// then builds and sends one message.
pub struct SmtpNotifier {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    subject: String,
}

impl SmtpNotifier {
    /// Build the notifier from its config, failing if the host or `From:` address
    /// is unusable — so a misconfiguration surfaces at boot, not on the first
    /// approval.
    pub fn new(config: SmtpConfig) -> Result<Self, NotifyError> {
        let from: Mailbox = config
            .from
            .parse()
            .map_err(|e| NotifyError::Delivery(format!("invalid From address: {e}")))?;

        let builder = if config.use_starttls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
        }
        .map_err(|e| NotifyError::Delivery(format!("SMTP relay setup: {e}")))?;

        let transport = builder
            .port(config.port)
            .credentials(Credentials::new(config.username, config.password))
            .build();

        Ok(Self {
            transport,
            from,
            subject: config.subject,
        })
    }
}

/// The plaintext body of the approval email.
fn body(accept_url: &str) -> String {
    format!(
        "You've been approved for an account on Democratos.\n\n\
         Follow this one-time link to finish setting up your account:\n\n\
         {accept_url}\n\n\
         The link works only once and expires after a while. If you didn't ask \
         for an account, you can ignore this email.\n"
    )
}

#[async_trait]
impl Notifier for SmtpNotifier {
    async fn notify_invite_approved(
        &self,
        to_email: &str,
        accept_url: &str,
    ) -> Result<(), NotifyError> {
        let to: Mailbox = to_email
            .parse()
            .map_err(|e| NotifyError::Delivery(format!("invalid recipient address: {e}")))?;

        let message = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(&self.subject)
            .body(body(accept_url))
            .map_err(|e| NotifyError::Delivery(format!("building message: {e}")))?;

        self.transport
            .send(message)
            .await
            .map_err(|e| NotifyError::Delivery(e.to_string()))?;
        Ok(())
    }
}
