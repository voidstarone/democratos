//! The canonical message a user signs to up/down/clear a post vote.

use crate::identity::domain::DOMAIN;

/// The canonical message a user signs to up/down/clear a post vote. `dir`:
/// `Some(true)` = up, `Some(false)` = down, `None` = clear.
pub fn post_vote_message(post: u64, dir: Option<bool>) -> String {
    let d = match dir {
        Some(true) => "up",
        Some(false) => "down",
        None => "clear",
    };
    format!("{DOMAIN}:post_vote:{post}:{d}")
}
