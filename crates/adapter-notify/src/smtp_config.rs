//! Connection + envelope settings for the SMTP notifier.

/// Everything [`SmtpNotifier`](crate::SmtpNotifier) needs to reach a mail server
/// and address a message. Plain data — the composition root fills it from CLI
/// flags / env and hands it over; no lettre types leak out here.
#[derive(Clone, Debug)]
pub struct SmtpConfig {
    /// SMTP server hostname.
    pub host: String,
    /// SMTP server port (typically 465 for implicit TLS, 587 for STARTTLS).
    pub port: u16,
    /// SMTP auth username.
    pub username: String,
    /// SMTP auth password.
    pub password: String,
    /// The `From:` address, e.g. `"Democratos <no-reply@example.org>"` or a bare
    /// `"no-reply@example.org"`.
    pub from: String,
    /// `true` → STARTTLS (upgrade a plaintext connection, port 587); `false` →
    /// implicit TLS from the first byte (port 465). Either way the session is
    /// encrypted — there is no cleartext option.
    pub use_starttls: bool,
    /// The `Subject:` line of the approval email.
    pub subject: String,
}
