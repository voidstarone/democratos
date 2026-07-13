//! The reviewer's classification form field.

use serde::Deserialize;

/// The tag a reviewer applies to a flagged item, as a [`SensitiveTag`] slug
/// (`"csam"`, `"gore"`, `"porn"`, …).
///
/// [`SensitiveTag`]: domain::SensitiveTag
#[derive(Deserialize)]
pub struct ClassifyForm {
    pub(crate) tag: String,
}
