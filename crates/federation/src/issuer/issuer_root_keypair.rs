//! The federation trust root's secret signing identity — held offline.

use ed25519_dalek::{Signer, SigningKey};

use crate::{from_hex, to_hex, FedError};

use super::issuer_cert::IssuerCert;
use super::issuer_cert_body::issuer_cert_body;
use super::issuer_root_public_key::IssuerRootPublicKey;

/// The federation trust root's secret signing identity. There is one per
/// federation, and it is the ultimate authority for *which servers may issue
/// accounts*. It is held **offline** by the federation operator — never on a node —
/// and used only by the certification tool to mint an
/// [`IssuerCert`](super::issuer_cert::IssuerCert) for a trusted node. The 32-byte
/// seed is the secret to guard.
pub struct IssuerRootKeypair {
    signing: SigningKey,
}

impl IssuerRootKeypair {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            signing: SigningKey::from_bytes(&seed),
        }
    }

    pub fn from_seed_hex(seed_hex: &str) -> Result<Self, FedError> {
        Ok(Self::from_seed(from_hex::<32>(seed_hex, "issuer root seed")?))
    }

    /// Fresh root identity from OS randomness — used once, when a federation is
    /// established. Guard the resulting [`seed_hex`](Self::seed_hex) offline.
    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).expect("OS randomness for issuer root keygen");
        Self::from_seed(seed)
    }

    pub fn public(&self) -> IssuerRootPublicKey {
        IssuerRootPublicKey::from_verifying(self.signing.verifying_key())
    }

    /// Hex 32-byte seed — persist as a SECRET, offline. Anyone holding it can
    /// certify a rogue node as a trusted account issuer.
    pub fn seed_hex(&self) -> String {
        to_hex(self.signing.to_bytes().as_slice())
    }

    /// Certify `node` as a trusted account issuer at `epoch`.
    pub fn certify(&self, node: u16, epoch: u64) -> IssuerCert {
        let body = issuer_cert_body(node, epoch);
        let sig = to_hex(&self.signing.sign(body.as_bytes()).to_bytes());
        IssuerCert { node, epoch, sig }
    }
}
