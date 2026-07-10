//! The composer's `?demos=` preselect query.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct SubmitQuery {
    /// Preselect this community (e.g. arriving from `/d/rust`).
    #[serde(default)]
    pub(crate) demos: Option<String>,
}
