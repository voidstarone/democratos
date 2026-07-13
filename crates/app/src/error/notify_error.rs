//! Why an outbound notification could not be delivered.

use thiserror::Error;

/// A notification-delivery failure. Adapters map their transport errors (SMTP,
/// IO, …) into [`Delivery`](NotifyError::Delivery); the caller surfaces it as a
/// "try again" — the approval itself is not recorded when the email can't go out.
#[derive(Debug, Error)]
pub enum NotifyError {
    #[error("notification delivery failed: {0}")]
    Delivery(String),
}
