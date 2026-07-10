//! The proposal-vote form fields.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct VoteForm {
    pub(crate) choice: String,
    /// The acting user's hex Ed25519 signature over the canonical vote message
    /// (`democratos:v1:vote:<proposal>:<aye|nay>`), produced on the client from the
    /// account's device-held key. Optional so accounts that haven't enrolled a key
    /// still work during rollout; required once the account has a key.
    #[serde(default)]
    pub(crate) signature: Option<String>,
}
