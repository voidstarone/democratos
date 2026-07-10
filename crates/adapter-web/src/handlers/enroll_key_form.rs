//! The enrol-signing-key form field.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct EnrollKeyForm {
    /// The account's hex Ed25519 **public** key. The browser generates the keypair
    /// (e.g. WebCrypto), keeps the secret on-device, and submits only this half.
    pub(crate) public_key: String,
}
