//! Form for assuming a trial-theater seat.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ActAsForm {
    /// The account id to become — must be a member of the demo court.
    pub(crate) id: u64,
    /// Where to land afterwards (validated same-site via `safe_next`).
    #[serde(default)]
    pub(crate) next: String,
}
