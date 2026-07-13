//! The `?tab=` query for the profile page.

use serde::Deserialize;

/// Which profile tab to show. Absent (or anything other than `comments`) means
/// the Posts tab.
#[derive(Deserialize)]
pub struct ProfileQuery {
    #[serde(default)]
    pub(crate) tab: Option<String>,
}
