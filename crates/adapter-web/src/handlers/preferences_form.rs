//! The preferences form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct PreferencesForm {
    /// `"auto"`, `"pages"`, or `"lazy"` — anything else is treated as `"auto"`.
    pub(crate) feed_paging: String,
    /// The sensitive-content reviewer opt-in. An unchecked HTML checkbox sends no
    /// field at all, so absence means "off"; `#[serde(default)]` maps that to
    /// `None`, and any present value (`"on"`) means "on".
    #[serde(default)]
    pub(crate) review_sensitive: Option<String>,
}
