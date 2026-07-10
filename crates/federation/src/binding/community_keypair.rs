//! A community's secret signing identity.

use ed25519_dalek::{Signer, SigningKey};

use crate::{from_hex, to_hex, CommunityPublicKey, FedError, HomeBinding};

use super::binding_body::binding_body;

/// A community's secret signing identity. Its home node holds this; the 32-byte
/// seed is persisted as a secret so the identity survives restarts and can re-sign
/// a binding when the community is re-homed.
pub struct CommunityKeypair {
    demos: u64,
    signing: SigningKey,
}

impl CommunityKeypair {
    pub fn from_seed(demos: u64, seed: [u8; 32]) -> Self {
        Self {
            demos,
            signing: SigningKey::from_bytes(&seed),
        }
    }

    pub fn from_seed_hex(demos: u64, seed_hex: &str) -> Result<Self, FedError> {
        Ok(Self::from_seed(
            demos,
            from_hex::<32>(seed_hex, "community seed")?,
        ))
    }

    /// Fresh identity from OS randomness — used once, when a community is founded.
    pub fn generate(demos: u64) -> Self {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).expect("OS randomness for community keygen");
        Self::from_seed(demos, seed)
    }

    pub fn demos(&self) -> u64 {
        self.demos
    }

    pub fn public(&self) -> CommunityPublicKey {
        CommunityPublicKey::from_verifying(self.demos, self.signing.verifying_key())
    }

    /// Hex 32-byte seed — persist as a SECRET (only the home node holds it).
    pub fn seed_hex(&self) -> String {
        to_hex(self.signing.to_bytes().as_slice())
    }

    /// Sign a home binding for this community. `allowed_failover` is normalised
    /// (sorted, de-duplicated) so the signed bytes are canonical.
    pub fn bind(&self, home_node: u16, allowed_failover: Vec<u16>, epoch: u64) -> HomeBinding {
        let mut failover = allowed_failover;
        failover.sort_unstable();
        failover.dedup();
        let body = binding_body(self.demos, home_node, &failover, epoch);
        let sig = to_hex(&self.signing.sign(body.as_bytes()).to_bytes());
        HomeBinding {
            demos: self.demos,
            home_node,
            allowed_failover: failover,
            epoch,
            sig,
        }
    }
}
