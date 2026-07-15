//! The trial-gallery comment form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct TrialCommentForm {
    pub(crate) body: String,
}
