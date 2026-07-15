//! One rendered row on the notifications page.

/// One notification, ready to render: a localized one-line summary linking to its
/// source, and whether it was still unseen when the page loaded (so the freshly
/// arrived ones can be highlighted before this visit marks them all seen).
pub struct NotificationRow {
    /// Same-site link to the source (a post, comment anchor, or trial).
    pub href: String,
    /// The localized summary line, e.g. "@alice mentioned you".
    pub summary: String,
    pub is_unseen: bool,
}
