//! The propose-rule form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct RuleForm {
    pub(crate) text: String,
}
