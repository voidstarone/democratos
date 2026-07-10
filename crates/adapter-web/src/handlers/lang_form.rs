//! The set-language form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct LangForm {
    pub(crate) lang: String,
}
