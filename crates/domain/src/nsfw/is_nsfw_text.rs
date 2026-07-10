//! The NSFW text-flagging predicate.

use crate::{nsfw_score, NSFW_FLAG_THRESHOLD};

/// Whether text scores at or above the flag threshold.
pub fn is_nsfw_text(text: &str) -> bool {
    nsfw_score(text) >= NSFW_FLAG_THRESHOLD
}
