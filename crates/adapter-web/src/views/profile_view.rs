//! The profile page: a user's posts or comments, tab-selected.

use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::profile_comment_item::ProfileCommentItem;
use crate::views::profile_post_item::ProfilePostItem;

/// A user's public profile at `/u/:handle`. Renders one tab at a time — the
/// active one is chosen by `?tab=` and carried in [`tab`](Self::tab) so the
/// template highlights it and the inactive list stays empty. Tabs are plain
/// links, so switching works with no JavaScript.
#[derive(Template)]
#[template(path = "profile.html")]
pub struct ProfileView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    /// The profile's owner (not the viewer).
    pub handle: String,
    /// `"posts"` or `"comments"`.
    pub tab: String,
    /// Whether to show the block/unblock control at all — a signed-in viewer
    /// looking at someone else's profile (never one's own).
    pub can_block: bool,
    /// Whether the viewer currently blocks this account (picks Unblock vs Block).
    pub is_blocked: bool,
    pub posts: Vec<ProfilePostItem>,
    pub comments: Vec<ProfileCommentItem>,
}
