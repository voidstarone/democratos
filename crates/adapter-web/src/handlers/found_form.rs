//! The found-a-community form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct FoundForm {
    /// The community's display name. Its URL slug is derived from this — the
    /// founder never types a slug by hand.
    pub(crate) name: String,
    /// Raw, comma/space-separated topic tags for the community. Normalized on
    /// submit (see [`domain::normalize_tags`]); optional, so it defaults to empty.
    #[serde(default)]
    pub(crate) tags: String,
}
