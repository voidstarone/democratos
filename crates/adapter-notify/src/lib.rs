//! Notification delivery adapters for [`app::Notifier`].
//!
//! Two implementations, chosen at the composition root:
//!
//! * [`SmtpNotifier`] — sends real email over authenticated SMTP (rustls TLS).
//! * [`LogNotifier`] — writes the message to the tracing log instead of sending
//!   it, so a dev box (or a node with no SMTP configured) still surfaces the
//!   one-time accept link for the operator to copy.
//!
//! A [`RecordingNotifier`] is also provided for tests that need to assert what
//! would have been sent.

pub mod log_notifier;
pub mod recording_notifier;
pub mod smtp_config;
pub mod smtp_notifier;

pub use log_notifier::LogNotifier;
pub use recording_notifier::RecordingNotifier;
pub use smtp_config::SmtpConfig;
pub use smtp_notifier::SmtpNotifier;
