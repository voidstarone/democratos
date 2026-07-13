//! The `?key=` secret carried on the admin review-queue page URL.

use serde::Deserialize;

/// The admin secret supplied as a query parameter on `GET /review-queue`, plus an
/// optional short outcome code (`msg`) an action redirect leaves for the page to
/// surface as a banner.
#[derive(Deserialize)]
pub struct AdminQuery {
    #[serde(default)]
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) msg: String,
}
