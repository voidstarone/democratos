use serde::Deserialize;

/// Optional `?key=` on `/dev/unlock`, checked against the configured unlock secret.
#[derive(Deserialize, Default)]
pub struct UnlockQuery {
    #[serde(default)]
    pub key: String,
}
