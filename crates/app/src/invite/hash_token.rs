//! Hash an invite token for storage/lookup.

use sha2::{Digest, Sha256};

/// The SHA-256 (lower-hex) digest of an invite token. Only this hash is ever
/// stored or queried, so a leaked waitlist yields no working invite links — the
/// same reason password hashes, not passwords, are stored. A plain unsalted
/// SHA-256 is enough here: the token itself is 256 bits of `OsRng` entropy (see
/// [`new_invite_token`](crate::invite::new_invite_token::new_invite_token)), so
/// it is not brute-forceable and needs no per-token salt or slow KDF.
pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        hex.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        hex.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_stable_64_char_hex() {
        let h = hash_token("some-token");
        assert_eq!(h.len(), 64);
        assert!(h.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(h, hash_token("some-token"), "hashing is deterministic");
    }

    #[test]
    fn distinct_tokens_hash_differently() {
        assert_ne!(hash_token("a"), hash_token("b"));
    }
}
