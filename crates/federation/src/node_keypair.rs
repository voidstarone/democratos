//! A node's secret signing identity.

use ed25519_dalek::{Signature, Signer, SigningKey};

use domain::NodeId;

use crate::{from_hex, to_hex, FedError, NodePublicKey};

/// A node's secret signing identity. Wraps an Ed25519 signing key; the 32-byte
/// seed is the thing to persist (keyfile / secret env), and it is [`zeroize`]d on
/// drop by `ed25519-dalek`.
pub struct NodeKeypair {
    node: NodeId,
    signing: SigningKey,
}

impl NodeKeypair {
    /// Load a node's identity from its persistent 32-byte seed.
    pub fn from_seed(node: NodeId, seed: [u8; 32]) -> Self {
        Self {
            node,
            signing: SigningKey::from_bytes(&seed),
        }
    }

    /// Load from a hex-encoded 32-byte seed (e.g. from a secret env var).
    pub fn from_seed_hex(node: NodeId, seed_hex: &str) -> Result<Self, FedError> {
        Ok(Self::from_seed(
            node,
            from_hex::<32>(seed_hex, "node seed")?,
        ))
    }

    /// Generate a fresh identity from OS randomness. **Keygen tooling / tests
    /// only** — a real node must load a persistent seed so its identity is stable.
    pub fn generate(node: NodeId) -> Self {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).expect("OS randomness for node keygen");
        Self::from_seed(node, seed)
    }

    pub fn node(&self) -> NodeId {
        self.node
    }

    /// This node's public key, to publish to peers / the control plane.
    pub fn public(&self) -> NodePublicKey {
        NodePublicKey::from_verifying(self.node, self.signing.verifying_key())
    }

    /// Hex seed — for persisting a freshly generated identity. Handle as a secret.
    pub fn seed_hex(&self) -> String {
        to_hex(self.signing.to_bytes().as_slice())
    }

    pub(crate) fn sign(&self, msg: &[u8]) -> Signature {
        self.signing.sign(msg)
    }

    /// Sign arbitrary bytes, returning a hex Ed25519 signature. Used to
    /// authenticate node-to-node **commands** (write intents forwarded to a
    /// community's owner) so the receiver attributes them to a control-plane-
    /// published node identity — not merely a shared symmetric bearer token.
    pub fn sign_hex(&self, msg: &[u8]) -> String {
        to_hex(&self.sign(msg).to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_hex_round_trips() {
        let kp = NodeKeypair::generate(NodeId(5));
        let pub_hex = kp.public().to_hex();
        let reparsed = NodePublicKey::from_hex(NodeId(5), &pub_hex).unwrap();
        assert_eq!(reparsed.to_hex(), pub_hex);
        // A persisted-then-reloaded seed is the same identity.
        let reloaded = NodeKeypair::from_seed_hex(NodeId(5), &kp.seed_hex()).unwrap();
        assert_eq!(reloaded.public().to_hex(), pub_hex);
    }
}
