//! The add-comment form fields.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct CommentForm {
    pub(crate) body: String,
    /// Set when replying to a specific comment (threaded reply).
    #[serde(default)]
    pub(crate) parent: Option<u64>,
}
