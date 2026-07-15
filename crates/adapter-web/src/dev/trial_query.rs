//! Query string for the trial-theater page.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct TrialQuery {
    /// Which seeded trial to show. Absent → the newest open case in the demo court.
    #[serde(default)]
    pub trial: Option<u64>,
}
