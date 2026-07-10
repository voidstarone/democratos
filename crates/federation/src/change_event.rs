//! A change event on the wire.

use ed25519_dalek::Signature;
use serde::{Deserialize, Serialize};

use domain::NodeId;

use crate::{from_hex, to_hex, FedError, NodeKeypair, NodePublicKey, SignedPart};

/// A change event on the wire.
///
/// The signed part travels as its **canonical JSON text** (`body`) — the exact
/// bytes that were signed — kept verbatim rather than reconstructed. A consumer
/// verifies the signature against the bytes it *actually received*, never against
/// a re-serialization of a parsed value. That closes a subtle footgun: relying on
/// `serde_json` to reproduce byte-identical output across nodes (different crate
/// versions, number formatting, or the `preserve_order` feature) would otherwise
/// silently break verification fleet-wide. Here, verification depends only on the
/// bytes on the wire and the key — nothing else.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// Canonical JSON of the [`SignedPart`] — the exact signed bytes.
    body: String,
    /// Hex-encoded Ed25519 signature over `body.as_bytes()`.
    signature: String,
}

impl ChangeEvent {
    /// Build and sign an event with `keypair`. The keypair's node is stamped into
    /// the part so the producing node can never be misdeclared.
    pub fn sign(keypair: &NodeKeypair, mut part: SignedPart) -> Self {
        part.node = keypair.node().0;
        let body = serde_json::to_string(&part).expect("SignedPart serializes");
        let sig = keypair.sign(body.as_bytes());
        Self {
            body,
            signature: to_hex(&sig.to_bytes()),
        }
    }

    /// Reconstruct an event from its wire fields (e.g. received over HTTP, or in a
    /// test). No validation happens here — call [`verify`](Self::verify).
    pub fn from_wire(body: String, signature: String) -> Self {
        Self { body, signature }
    }

    /// The canonical signed bytes, verbatim.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// The hex signature.
    pub fn signature(&self) -> &str {
        &self.signature
    }

    /// Verify this event was signed by `key`, returning the parsed signed part
    /// **only** after the signature over the received bytes checks out.
    ///
    /// The caller is still responsible for the *authorization* checks a signature
    /// cannot make: that `key` belongs to the rightful owner of `part.demos` at
    /// `part.epoch`, and that `part.seq` is past the consumer's cursor.
    pub fn verify(&self, key: &NodePublicKey) -> Result<SignedPart, FedError> {
        let sig_bytes = from_hex::<64>(&self.signature, "signature")?;
        let sig = Signature::from_bytes(&sig_bytes);
        // Verify against the exact bytes received — not a re-serialization.
        key.verify(self.body.as_bytes(), &sig)?;
        let part: SignedPart = serde_json::from_str(&self.body).map_err(|_| FedError::BadBody)?;
        if key.node() != NodeId(part.node) {
            return Err(FedError::BadSignature);
        }
        Ok(part)
    }

    /// Read the signed part **without** verifying — for routing/telemetry only
    /// (e.g. to learn which community an event scopes to). Never trust its
    /// contents for an authorization decision.
    pub fn peek(&self) -> Result<SignedPart, FedError> {
        serde_json::from_str(&self.body).map_err(|_| FedError::BadBody)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChangeOp;

    fn part() -> SignedPart {
        SignedPart {
            node: 0, // overwritten by sign()
            epoch: 3,
            seq: 42,
            demos: Some(7),
            entity: "posts".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "id": 123, "title": "hello" }),
        }
    }

    #[test]
    fn a_signed_event_verifies_and_carries_the_signing_node() {
        let kp = NodeKeypair::generate(NodeId(9));
        let ev = ChangeEvent::sign(&kp, part());
        assert_eq!(
            ev.peek().unwrap().node,
            9,
            "sign() stamps the keypair's node"
        );
        let verified = ev.verify(&kp.public()).expect("verifies");
        assert_eq!(verified.seq, 42);
    }

    #[test]
    fn a_tampered_payload_is_rejected() {
        let kp = NodeKeypair::generate(NodeId(9));
        let ev = ChangeEvent::sign(&kp, part());
        // Attacker rewrites the row in the signed body after signing.
        let forged = ChangeEvent::from_wire(
            ev.body().replace("hello", "HIJACKED"),
            ev.signature().to_string(),
        );
        assert_eq!(forged.verify(&kp.public()), Err(FedError::BadSignature));
    }

    #[test]
    fn tampering_with_any_signed_field_is_rejected() {
        let kp = NodeKeypair::generate(NodeId(9));
        // Each mutation rewrites one field in the signed body; all must fail.
        for (from, to) in [
            ("\"epoch\":3", "\"epoch\":999"),
            ("\"seq\":42", "\"seq\":1"),
            ("\"demos\":7", "\"demos\":8"),
            ("\"entity\":\"posts\"", "\"entity\":\"votes\""),
        ] {
            let ev = ChangeEvent::sign(&kp, part());
            let body = ev.body().replace(from, to);
            assert_ne!(body, ev.body(), "the tamper actually changed the body");
            let forged = ChangeEvent::from_wire(body, ev.signature().to_string());
            assert_eq!(forged.verify(&kp.public()), Err(FedError::BadSignature));
        }
    }

    #[test]
    fn verification_uses_transmitted_bytes_not_a_reserialization() {
        // Re-encoding the body with reordered keys (as a lax proxy would) must not
        // change the verdict: verification is over the *received* bytes only.
        let kp = NodeKeypair::generate(NodeId(9));
        let ev = ChangeEvent::sign(&kp, part());
        // Any byte change to the body invalidates the signature — even a
        // whitespace tweak — proving verification is byte-exact, not semantic.
        let with_space =
            ChangeEvent::from_wire(format!("{} ", ev.body()), ev.signature().to_string());
        assert_eq!(with_space.verify(&kp.public()), Err(FedError::BadSignature));
        // And the pristine event still verifies.
        assert!(ev.verify(&kp.public()).is_ok());
    }

    #[test]
    fn another_nodes_key_does_not_verify() {
        let owner = NodeKeypair::generate(NodeId(9));
        let attacker = NodeKeypair::generate(NodeId(9)); // same claimed node, different key
        let ev = ChangeEvent::sign(&owner, part());
        assert_eq!(ev.verify(&attacker.public()), Err(FedError::BadSignature));
    }

    #[test]
    fn a_forged_signature_is_rejected() {
        let kp = NodeKeypair::generate(NodeId(9));
        let ev = ChangeEvent::sign(&kp, part());
        let forged = ChangeEvent::from_wire(ev.body().to_string(), to_hex(&[0u8; 64]));
        assert_eq!(forged.verify(&kp.public()), Err(FedError::BadSignature));
    }

    #[test]
    fn a_malformed_body_is_rejected_not_panicked() {
        let kp = NodeKeypair::generate(NodeId(9));
        // Signed, but the body is not valid SignedPart JSON.
        let sig = kp.public(); // just to have a key
        let ev = ChangeEvent::from_wire("not json".into(), to_hex(&[0u8; 64]));
        assert!(matches!(
            ev.verify(&sig),
            Err(FedError::BadSignature) | Err(FedError::BadBody)
        ));
        assert_eq!(ev.peek(), Err(FedError::BadBody));
    }
}
