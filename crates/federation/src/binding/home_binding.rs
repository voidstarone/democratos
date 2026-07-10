//! A founder-signed declaration of a community's home node and failover set.

use serde::{Deserialize, Serialize};

use crate::{CommunityPublicKey, FedError};

use super::binding_body::binding_body;

/// A founder-signed declaration of a community's home node and its pre-authorized
/// failover set, signed by the community's
/// [`CommunityKeypair`](super::community_keypair::CommunityKeypair). A higher
/// `epoch` supersedes a lower one (used to migrate the home to a new node).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeBinding {
    pub demos: u64,
    pub home_node: u16,
    pub allowed_failover: Vec<u16>,
    pub epoch: u64,
    /// Ed25519 signature (hex) by the community key over `binding_body`.
    pub sig: String,
}

impl HomeBinding {
    /// Verify this binding was signed by `key` (the community's public key) over
    /// its own contents. Any tamper (home node, failover set, epoch) fails.
    pub fn verify(&self, key: &CommunityPublicKey) -> Result<(), FedError> {
        if key.demos() != self.demos {
            return Err(FedError::BadSignature);
        }
        let body = binding_body(self.demos, self.home_node, &self.allowed_failover, self.epoch);
        key.verify(body.as_bytes(), &self.sig)
    }

    /// Whether `node` is authorized to OWN the community under this binding — the
    /// home node itself, or a pre-authorized failover heir.
    pub fn authorizes(&self, node: u16) -> bool {
        self.home_node == node || self.allowed_failover.contains(&node)
    }
}

#[cfg(test)]
mod tests {
    use crate::CommunityKeypair;

    #[test]
    fn a_binding_verifies_and_authorizes_home_and_failover() {
        let kp = CommunityKeypair::generate(0x0007_0000_0000_0001);
        let pubkey = kp.public();
        let binding = kp.bind(7, vec![8, 9], 1);

        assert!(binding.verify(&pubkey).is_ok());
        assert!(binding.authorizes(7), "home node is authorized");
        assert!(binding.authorizes(8), "a failover node is authorized");
        assert!(!binding.authorizes(42), "an outsider is NOT authorized");
    }

    #[test]
    fn tampering_breaks_the_signature() {
        let kp = CommunityKeypair::generate(0x0007_0000_0000_0001);
        let pubkey = kp.public();
        let mut binding = kp.bind(7, vec![8], 1);

        // Seize attempt: rewrite the home node to a node not signed for.
        binding.home_node = 42;
        assert!(binding.verify(&pubkey).is_err(), "a forged home node fails verify");

        // Add yourself to the failover set.
        let mut b2 = kp.bind(7, vec![8], 1);
        b2.allowed_failover.push(42);
        assert!(b2.verify(&pubkey).is_err(), "a padded failover set fails verify");
    }

    #[test]
    fn a_different_community_key_does_not_verify() {
        let kp = CommunityKeypair::generate(0x0007_0000_0000_0001);
        let other = CommunityKeypair::generate(0x0007_0000_0000_0001);
        let binding = kp.bind(7, vec![], 1);
        assert!(binding.verify(&other.public()).is_err());
    }
}
