//! The federation trust root's public key — baked into every node's config.

use ed25519_dalek::{Signature, VerifyingKey};

use crate::{from_hex, to_hex, FedError};

/// The federation trust root's public key. Every node ships this in its config
/// (`FEDERATION_TRUST_ROOT`); it is the single anchor against which a node's
/// [`IssuerCert`](super::issuer_cert::IssuerCert) is verified. There is exactly one
/// root per federation, so — unlike a community key — it is not scoped to any id.
/// The matching secret ([`IssuerRootKeypair`](super::issuer_root_keypair::IssuerRootKeypair))
/// is held offline and only ever used by the certification tool.
#[derive(Clone)]
pub struct IssuerRootPublicKey {
    verifying: VerifyingKey,
}

impl IssuerRootPublicKey {
    /// Build from an already-parsed verifying key.
    pub(crate) fn from_verifying(verifying: VerifyingKey) -> Self {
        Self { verifying }
    }

    pub fn from_hex(hex: &str) -> Result<Self, FedError> {
        let bytes = from_hex::<32>(hex, "issuer root public key")?;
        let verifying = VerifyingKey::from_bytes(&bytes).map_err(|_| FedError::BadKey)?;
        Ok(Self { verifying })
    }

    pub fn to_hex(&self) -> String {
        to_hex(self.verifying.to_bytes().as_slice())
    }

    pub(crate) fn verify(&self, msg: &[u8], sig_hex: &str) -> Result<(), FedError> {
        let sig_bytes = from_hex::<64>(sig_hex, "issuer cert signature")?;
        let sig = Signature::from_bytes(&sig_bytes);
        // `verify_strict` rejects non-canonical / small-order signatures.
        self.verifying
            .verify_strict(msg, &sig)
            .map_err(|_| FedError::BadSignature)
    }
}
