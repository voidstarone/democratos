//! Which notification adapter the composition root wires.

use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum NotifierKind {
    /// Log the invite link instead of emailing it — dev / no-SMTP fallback.
    Log,
    /// Send real email over authenticated SMTP (requires `--smtp-*`).
    Smtp,
}
