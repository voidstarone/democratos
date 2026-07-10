//! Whether a hex string is a well-formed Ed25519 public key.

use crate::identity::user_public_key::UserPublicKey;

/// Whether `hex` is a well-formed Ed25519 public key (32 bytes, valid point).
pub fn is_valid_public_key(hex: &str) -> bool {
    UserPublicKey::from_hex(hex).is_some()
}
