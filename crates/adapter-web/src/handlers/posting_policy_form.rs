//! The change-posting-policy form fields.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct PostingPolicyForm {
    /// `open` | `members` | `voters` | `min`.
    pub(crate) policy: String,
    /// Popularity threshold, used only when `policy == "min"`.
    #[serde(default)]
    pub(crate) threshold: Option<i64>,
}
