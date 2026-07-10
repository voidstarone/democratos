//! Signs and verifies session cookie values, keyed by a server-held secret.

use std::sync::Arc;

use hmac::{Hmac, Mac};
use rand_core::{OsRng, RngCore};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Signs and verifies session cookie values, keyed by a server-held secret.
///
/// Cheap to clone (the key is shared behind an `Arc`), so it can live in the
/// per-request handler state.
#[derive(Clone)]
pub struct SessionSigner {
    key: Arc<[u8]>,
}

impl SessionSigner {
    /// Build a signer from a caller-supplied secret (e.g. an env var), so that
    /// signed cookies survive a restart and are valid across every node that
    /// shares the secret. The secret should be long and random; HMAC accepts a
    /// key of any length, but short keys weaken it.
    pub fn from_secret(secret: &[u8]) -> Self {
        Self {
            key: Arc::from(secret),
        }
    }

    /// Build a signer with a fresh random 256-bit key. Convenient and fully
    /// secure, but the key is ephemeral: existing sessions are invalidated on
    /// restart, and it is not shared across a federated cluster. Prefer
    /// [`from_secret`](Self::from_secret) in any real deployment.
    pub fn ephemeral() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self {
            key: Arc::from(&key[..]),
        }
    }

    /// The signed cookie value for a user id and its absolute expiry (a Unix
    /// timestamp, in seconds): `"<id>.<expires_at>.<hex-mac>"`. The tag covers
    /// *both* id and expiry, so neither can be altered — a browser can't extend
    /// its own session by editing the expiry, and a stolen cookie stops working
    /// once its expiry passes (the caller enforces that in [`verify`](Self::verify)).
    pub fn sign(&self, uid: u64, expires_at: i64) -> String {
        format!("{uid}.{expires_at}.{}", self.tag(uid, expires_at))
    }

    /// Recover `(user id, expires_at)` from a signed cookie value, or `None` if the
    /// value is malformed or its tag doesn't verify. Comparison is constant-time,
    /// so a forged cookie leaks nothing about how close a guess was. This does
    /// **not** check the clock — the caller compares `expires_at` against now, so
    /// the pure signer stays clock-free (and testable).
    pub fn verify(&self, value: &str) -> Option<(u64, i64)> {
        let (id_str, rest) = value.split_once('.')?;
        let (exp_str, mac_hex) = rest.split_once('.')?;
        let uid: u64 = id_str.parse().ok()?;
        let expires_at: i64 = exp_str.parse().ok()?;
        let provided = decode_hex(mac_hex)?;
        // `verify_slice` is a constant-time comparison and errors on any mismatch
        // (including a wrong-length tag), so both arms collapse to `None`.
        self.mac_for(uid, expires_at)
            .verify_slice(&provided)
            .ok()
            .map(|()| (uid, expires_at))
    }

    /// The hex-encoded HMAC tag over a user id and its expiry.
    fn tag(&self, uid: u64, expires_at: i64) -> String {
        encode_hex(&self.mac_for(uid, expires_at).finalize().into_bytes())
    }

    /// A fresh keyed MAC primed with the id and expiry bytes, ready to finalize or
    /// verify. Both fields are length-fixed (8 bytes each), so their concatenation
    /// is unambiguous — no separator needed for the MAC input.
    fn mac_for(&self, uid: u64, expires_at: i64) -> HmacSha256 {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(&uid.to_le_bytes());
        mac.update(&expires_at.to_le_bytes());
        mac
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    s.as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16)?;
            let lo = (pair[1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_signed_id() {
        let signer = SessionSigner::from_secret(b"a-test-secret-key");
        let cookie = signer.sign(42, 1_000);
        assert_eq!(signer.verify(&cookie), Some((42, 1_000)));
    }

    #[test]
    fn rejects_a_forged_id() {
        // The classic attack: hand-write `uid=1` with no valid tag.
        let signer = SessionSigner::from_secret(b"a-test-secret-key");
        assert_eq!(signer.verify("1"), None);
        assert_eq!(signer.verify("1."), None);
        assert_eq!(signer.verify("1.deadbeef"), None);
        assert_eq!(signer.verify("1.1000.deadbeef"), None);
        assert_eq!(signer.verify("garbage"), None);
    }

    #[test]
    fn rejects_a_tag_swapped_onto_another_id_or_expiry() {
        // A valid tag for (user 1, exp 1000) must not authenticate a different id
        // or a stretched expiry — the tag binds both.
        let signer = SessionSigner::from_secret(b"a-test-secret-key");
        let one = signer.sign(1, 1_000);
        let tag = one.rsplit_once('.').unwrap().1;
        assert_eq!(signer.verify(&format!("2.1000.{tag}")), None);
        assert_eq!(signer.verify(&format!("1.9999.{tag}")), None);
    }

    #[test]
    fn a_cookie_from_another_key_is_rejected() {
        let issuer = SessionSigner::from_secret(b"issuer-secret");
        let attacker = SessionSigner::from_secret(b"different-secret");
        let cookie = issuer.sign(7, 1_000);
        assert_eq!(attacker.verify(&cookie), None);
        assert_eq!(issuer.verify(&cookie), Some((7, 1_000)));
    }
}
