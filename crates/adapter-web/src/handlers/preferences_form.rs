//! The preferences form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct PreferencesForm {
    /// `"auto"`, `"pages"`, or `"lazy"` — anything else is treated as `"auto"`.
    pub(crate) feed_paging: String,
}
