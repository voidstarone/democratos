//! What a panel is judging.

use serde::{Deserialize, Serialize};

/// What a panel is judging. Comments draw a smaller panel than posts — a comment
/// is lower-stakes than a top-level post — while a user-level report (e.g. a
/// suspected bot) is treated as the heavier, post-weight case.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ContentScale {
    Post,
    Comment,
}
