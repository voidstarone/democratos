//! The propose-rule form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct RuleForm {
    pub(crate) text: String,
    /// The ban term (days) a conviction for breaking this rule carries. `0` (the
    /// default when the field is blank/absent) means "inherit the community
    /// ceiling". Clamped to the community ceiling when the proposal is enacted.
    #[serde(default)]
    pub(crate) sanction_days: u32,
}
