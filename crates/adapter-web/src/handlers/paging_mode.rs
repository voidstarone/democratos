//! The effective feed-delivery mode for a render.

use domain::User;

use crate::handlers::paging_str::paging_str;

/// The effective feed-delivery mode for a render: the signed-in viewer's saved
/// account preference, or [`FeedPaging::Auto`](domain::FeedPaging::Auto) for a
/// signed-out viewer.
pub(crate) fn paging_mode(user: Option<&User>) -> &'static str {
    paging_str(user.map(|u| u.feed_paging).unwrap_or_default())
}
