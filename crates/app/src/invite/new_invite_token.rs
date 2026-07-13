//! Mint a fresh, unguessable invite token.

use rand_core::{OsRng, RngCore};

/// A new single-use invite token: 256 bits of OS entropy, lower-hex encoded (64
/// chars, URL-safe). This is the *raw* secret that goes in the email link and is
/// never stored — only its [`hash_token`](crate::invite::hash_token::hash_token)
/// digest is persisted.
pub fn new_invite_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let mut hex = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        hex.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        hex.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_64_char_hex_and_unpredictable() {
        let a = new_invite_token();
        let b = new_invite_token();
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b, "two draws must not collide");
    }
}
