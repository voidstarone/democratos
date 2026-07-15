//! The set-ban-ceiling form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct MaxSanctionForm {
    /// The proposed community ban ceiling, in days. Clamped to the 18-year
    /// platform cap when the proposal is enacted.
    pub(crate) days: u32,
}
