//! The amend-criteria form fields.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct AmendForm {
    pub(crate) min_account_age_days: i64,
    pub(crate) min_membership_days: i64,
    pub(crate) min_contribution: i64,
}
