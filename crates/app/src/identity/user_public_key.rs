//! A user's Ed25519 **public** verifying key — verify-only.

use ed25519_dalek::{Signature, VerifyingKey};

/// A user's Ed25519 **public** verifying key, parsed from hex. Verify-only: this
/// type cannot produce a signature, only check one.
pub struct UserPublicKey(VerifyingKey);

impl UserPublicKey {
    /// Parse from a hex-encoded 32-byte Ed25519 public key, or `None` if the hex
    /// is malformed or not a valid curve point.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let bytes = decode_hex::<32>(hex)?;
        VerifyingKey::from_bytes(&bytes).ok().map(Self)
    }

    /// Whether `sig_hex` is a valid signature by this key over `message`.
    /// Uses strict verification (rejects non-canonical / small-order signatures),
    /// matching the federation node-key hardening.
    pub fn verify(&self, message: &str, sig_hex: &str) -> bool {
        let Some(sig_bytes) = decode_hex::<64>(sig_hex) else {
            return false;
        };
        let sig = Signature::from_bytes(&sig_bytes);
        self.0.verify_strict(message.as_bytes(), &sig).is_ok()
    }
}

fn decode_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let bytes = s.as_bytes();
    if bytes.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (i, chunk) in bytes.chunks_exact(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out[i] = ((hi << 4) | lo) as u8;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::is_valid_public_key::is_valid_public_key;
    use crate::identity::post_vote_message::post_vote_message;
    use crate::identity::vote_message::vote_message;
    use ed25519_dalek::{Signer, SigningKey};

    /// A throwaway user keypair for tests, standing in for a device-held secret.
    fn keypair(seed: u8) -> (SigningKey, String) {
        let sk = SigningKey::from_bytes(&[seed; 32]);
        let pub_hex = encode_hex(sk.verifying_key().as_bytes());
        (sk, pub_hex)
    }

    fn sign(sk: &SigningKey, message: &str) -> String {
        encode_hex(&sk.sign(message.as_bytes()).to_bytes())
    }

    fn encode_hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
            s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
        }
        s
    }

    #[test]
    fn a_users_own_signature_over_the_action_verifies() {
        let (sk, pub_hex) = keypair(1);
        let key = UserPublicKey::from_hex(&pub_hex).unwrap();
        let msg = vote_message(42, true);
        assert!(key.verify(&msg, &sign(&sk, &msg)));
    }

    #[test]
    fn a_signature_for_a_different_decision_does_not_verify() {
        // A signed "aye" must not authorize a "nay" (nor another proposal): the
        // message — and therefore the signature — is bound to the exact action.
        let (sk, pub_hex) = keypair(2);
        let key = UserPublicKey::from_hex(&pub_hex).unwrap();
        let aye_sig = sign(&sk, &vote_message(42, true));
        assert!(key.verify(&vote_message(42, true), &aye_sig));
        assert!(
            !key.verify(&vote_message(42, false), &aye_sig),
            "aye != nay"
        );
        assert!(
            !key.verify(&vote_message(43, true), &aye_sig),
            "proposal is bound"
        );
    }

    #[test]
    fn another_users_key_does_not_verify_the_signature() {
        // The forgery an open federation must stop: a node signs a victim's vote.
        let (victim_sk, _) = keypair(3);
        let (_, attacker_pub) = keypair(4);
        let attacker_key = UserPublicKey::from_hex(&attacker_pub).unwrap();
        let msg = vote_message(7, true);
        assert!(!attacker_key.verify(&msg, &sign(&victim_sk, &msg)));
    }

    #[test]
    fn cross_action_replay_is_blocked_by_domain_separation() {
        // A signature captured for a post vote can't be replayed as a proposal
        // ballot even if the ids collide, because the tag differs.
        let (sk, pub_hex) = keypair(5);
        let key = UserPublicKey::from_hex(&pub_hex).unwrap();
        let post_sig = sign(&sk, &post_vote_message(9, Some(true)));
        assert!(!key.verify(&vote_message(9, true), &post_sig));
    }

    #[test]
    fn malformed_keys_and_signatures_are_rejected_not_panicked() {
        assert!(UserPublicKey::from_hex("not-hex").is_none());
        assert!(UserPublicKey::from_hex("").is_none());
        assert!(!is_valid_public_key("00"));
        let (_, pub_hex) = keypair(6);
        let key = UserPublicKey::from_hex(&pub_hex).unwrap();
        assert!(!key.verify(&vote_message(1, true), "garbage"));
        assert!(!key.verify(&vote_message(1, true), ""));
    }
}
