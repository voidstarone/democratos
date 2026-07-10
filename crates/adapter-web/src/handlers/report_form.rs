//! The report-post form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ReportForm {
    /// The id of the community rule the reporter says the post breaks. Parsed
    /// leniently from a string so an absent/blank dropdown value becomes `None`
    /// rather than a 400.
    #[serde(default)]
    pub(crate) rule: Option<String>,
}
