use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::invite_queue_item::InviteQueueItem;

/// The operator's invite review queue — reachable only from an allowed subnet
/// with the admin secret. Lists pending requests with approve/reject actions and
/// the live invitation-only toggle.
#[derive(Template)]
#[template(path = "review_queue.html")]
pub struct ReviewQueueView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    /// The admin secret, threaded into every action form and the self-link so the
    /// operator stays authenticated across clicks.
    pub key: String,
    pub csrf_token: String,
    /// Whether invitation-only is currently on — drives the toggle.
    pub invite_only: bool,
    /// A short outcome code from the last action (e.g. `"approved"`), or empty.
    /// The template maps it to a friendly banner.
    pub msg: String,
    pub items: Vec<InviteQueueItem>,
}

impl ReviewQueueView {
    /// The banner text for the last action's outcome code and whether it is a
    /// failure (styled red), or `None` when there is nothing to show.
    pub(crate) fn banner(&self) -> Option<(&'static str, bool)> {
        let msg = match self.msg.as_str() {
            "approved" => ("Approved — invite email sent.", false),
            "email-failed" => (
                "Could not send the invite email — the request is still pending. \
                 Check the SMTP settings (or notifier) and try again.",
                true,
            ),
            "rejected" => ("Request rejected.", false),
            "not-pending" => ("That request was already handled.", true),
            "invite-on" => ("Invitation-only is now ON.", false),
            "invite-off" => ("Invitation-only is now OFF.", false),
            "csrf" => ("Your session expired — please try that again.", true),
            "error" => ("Something went wrong — please try again.", true),
            _ => return None,
        };
        Some(msg)
    }
}
