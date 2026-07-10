//! The propose-removal form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct RemoveForm {
    pub(crate) target: String,
}
