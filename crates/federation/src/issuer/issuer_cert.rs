//! A federation-root-signed certificate naming a trusted account issuer.

use serde::{Deserialize, Serialize};

use crate::FedError;

use super::issuer_cert_body::issuer_cert_body;
use super::issuer_root_public_key::IssuerRootPublicKey;

/// A federation-root-signed statement that `node` is a **trusted account issuer** —
/// a server whose minted user accounts are honoured across every community. Signed
/// by the [`IssuerRootKeypair`](super::issuer_root_keypair::IssuerRootKeypair) over
/// [`issuer_cert_body`]. A higher `epoch` supersedes a lower one (issuer-key
/// rotation); to revoke trust, the cert is removed from the control plane.
///
/// This is the anchor that [`authorize`](crate::authorize) consults on a global
/// (user-account) event: only a node holding a valid cert may create accounts that
/// replicate fleet-wide. A node cannot forge one without the federation root's
/// secret key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuerCert {
    pub node: u16,
    pub epoch: u64,
    /// Ed25519 signature (hex) by the federation root key over `issuer_cert_body`.
    pub sig: String,
}

impl IssuerCert {
    /// Verify this cert was signed by the federation `root` over its own contents.
    /// Any tamper (node, epoch) fails.
    pub fn verify(&self, root: &IssuerRootPublicKey) -> Result<(), FedError> {
        let body = issuer_cert_body(self.node, self.epoch);
        root.verify(body.as_bytes(), &self.sig)
    }

    /// Whether this cert certifies `node` as a trusted issuer.
    pub fn certifies(&self, node: u16) -> bool {
        self.node == node
    }
}

#[cfg(test)]
mod tests {
    use crate::IssuerRootKeypair;

    #[test]
    fn a_cert_verifies_and_certifies_its_named_node() {
        let root = IssuerRootKeypair::generate();
        let pubkey = root.public();
        let cert = root.certify(4, 1);

        assert!(cert.verify(&pubkey).is_ok());
        assert!(cert.certifies(4), "the named node is certified");
        assert!(!cert.certifies(9), "a different node is NOT certified");
    }

    #[test]
    fn tampering_breaks_the_signature() {
        let root = IssuerRootKeypair::generate();
        let pubkey = root.public();

        // Promote a different node into a cert signed for node 4.
        let mut forged = root.certify(4, 1);
        forged.node = 9;
        assert!(forged.verify(&pubkey).is_err(), "a swapped node fails verify");

        // Bump the epoch to look like a newer grant.
        let mut bumped = root.certify(4, 1);
        bumped.epoch = 9_999;
        assert!(bumped.verify(&pubkey).is_err(), "a swapped epoch fails verify");
    }

    #[test]
    fn a_different_root_key_does_not_verify() {
        let root = IssuerRootKeypair::generate();
        let impostor = IssuerRootKeypair::generate();
        let cert = root.certify(4, 1);
        assert!(
            cert.verify(&impostor.public()).is_err(),
            "only the true federation root can certify an issuer"
        );
    }
}
