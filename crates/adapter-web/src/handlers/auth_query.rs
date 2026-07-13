//! Query string for the sign-in / register pages.

use serde::Deserialize;

/// The auth pages accept an optional `?next=` — where to send the visitor once
/// they authenticate (e.g. back to the `/found/:id` petition they came from).
/// Validated through [`safe_next`](crate::handlers::safe_next::safe_next) before
/// use; never trusted as-is.
#[derive(Deserialize)]
pub struct AuthQuery {
    #[serde(default)]
    pub next: Option<String>,
}
