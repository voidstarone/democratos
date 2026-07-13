//! The `?token=` query on the invite-accept link.

use serde::Deserialize;

/// The one-time token carried on `GET /invite/accept?token=…`.
#[derive(Deserialize)]
pub struct AcceptQuery {
    #[serde(default)]
    pub(crate) token: String,
}
