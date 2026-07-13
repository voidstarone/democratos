//! Build the chosen notification adapter from the CLI settings.

use std::sync::Arc;

use anyhow::{Context, Result};

use adapter_notify::{LogNotifier, SmtpConfig, SmtpNotifier};
use app::Notifier;

use crate::cli::Cli;
use crate::notifier_kind::NotifierKind;

/// Wire the invite-email notifier. `log` never fails; `smtp` fails closed if its
/// required settings (host, username, password, from) are missing, so a
/// misconfigured mail setup is caught at boot rather than on the first approval.
pub(crate) fn build_notifier(cli: &Cli) -> Result<Arc<dyn Notifier>> {
    match cli.notifier {
        NotifierKind::Log => Ok(Arc::new(LogNotifier::new())),
        NotifierKind::Smtp => {
            let host = cli
                .smtp_host
                .clone()
                .context("--notifier smtp requires --smtp-host (DEMOCRATOS_SMTP_HOST)")?;
            let username = cli
                .smtp_username
                .clone()
                .context("--notifier smtp requires --smtp-username (DEMOCRATOS_SMTP_USERNAME)")?;
            let password = cli
                .smtp_password
                .clone()
                .context("--notifier smtp requires --smtp-password (DEMOCRATOS_SMTP_PASSWORD)")?;
            let from = cli
                .smtp_from
                .clone()
                .context("--notifier smtp requires --smtp-from (DEMOCRATOS_SMTP_FROM)")?;

            let notifier = SmtpNotifier::new(SmtpConfig {
                host,
                port: cli.smtp_port,
                username,
                password,
                from,
                use_starttls: cli.smtp_starttls,
                subject: cli.smtp_subject.clone(),
            })
            .map_err(|e| anyhow::anyhow!("SMTP notifier: {e}"))?;
            Ok(Arc::new(notifier))
        }
    }
}
