//! The found-a-community form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct FoundForm {
    /// The community's display name. Its URL slug is derived from this — the
    /// founder never types a slug by hand.
    pub(crate) name: String,
}
