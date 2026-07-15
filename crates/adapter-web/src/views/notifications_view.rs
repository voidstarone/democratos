//! The notifications page: a member's recent pings, newest first.

use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::notification_row::NotificationRow;

/// A member's notifications at `/notifications`. Opening the page marks them all
/// seen (clearing the toolbar badge); rows that were still unseen on arrival are
/// flagged so the template can mark them.
#[derive(Template)]
#[template(path = "notifications.html")]
pub struct NotificationsView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub rows: Vec<NotificationRow>,
}
