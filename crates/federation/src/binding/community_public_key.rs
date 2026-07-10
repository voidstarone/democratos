//! A community's public key — safe to publish.

use ed25519_dalek::{Signature, VerifyingKey};

use crate::{from_hex, to_hex, FedError};

/// A community's public key — safe to publish. Peers verify home bindings with it.
#[derive(Clone)]
pub struct CommunityPublicKey {
    demos: u64,
    verifying: VerifyingKey,
}

impl CommunityPublicKey {
    /// Build from a demos id and its already-parsed verifying key.
    pub(crate) fn from_verifying(demos: u64, verifying: VerifyingKey) -> Self {
        Self { demos, verifying }
    }

    pub fn from_hex(demos: u64, hex: &str) -> Result<Self, FedError> {
        let bytes = from_hex::<32>(hex, "community public key")?;
        let verifying = VerifyingKey::from_bytes(&bytes).map_err(|_| FedError::BadKey)?;
        Ok(Self { demos, verifying })
    }

    pub fn demos(&self) -> u64 {
        self.demos
    }

    pub fn to_hex(&self) -> String {
        to_hex(self.verifying.to_bytes().as_slice())
    }

    pub(crate) fn verify(&self, msg: &[u8], sig_hex: &str) -> Result<(), FedError> {
        let sig_bytes = from_hex::<64>(sig_hex, "binding signature")?;
        let sig = Signature::from_bytes(&sig_bytes);
        // `verify_strict` rejects non-canonical / small-order signatures.
        self.verifying
            .verify_strict(msg, &sig)
            .map_err(|_| FedError::BadSignature)
    }
}
