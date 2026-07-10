//! The jury-verdict form fields.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct VerdictForm {
    pub(crate) verdict: String,
    /// The juror's signature over the canonical jury-vote message. Optional during
    /// rollout; required once the juror's account has enrolled a key.
    #[serde(default)]
    pub(crate) signature: Option<String>,
}
