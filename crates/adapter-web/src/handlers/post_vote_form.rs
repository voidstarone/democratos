//! The post/comment up-down vote form fields.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct PostVoteForm {
    pub(crate) dir: String, // "up" | "down"
    /// The acting user's signature over the canonical post-vote message for the
    /// *resolved* direction (which the client computes from the vote it rendered).
    /// Optional during rollout; required once the account has enrolled a key. Only
    /// consulted for post votes (comment votes remain a local, unsigned action).
    #[serde(default)]
    pub(crate) signature: Option<String>,
}
