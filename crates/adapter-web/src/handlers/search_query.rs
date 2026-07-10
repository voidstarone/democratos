//! The search-page query parameters.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub(crate) q: String,
    /// `"all"`, empty, or a community slug.
    #[serde(default)]
    pub(crate) scope: String,
    #[serde(default)]
    pub(crate) tag: Option<String>,
    #[serde(default)]
    pub(crate) page: Option<u32>,
}
