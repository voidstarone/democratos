//! Federation-root-signed **trusted issuer** certificates — the cryptographic
//! anchor that makes "only trusted servers may create fleet-wide accounts"
//! enforceable in an **open federation of untrusted node operators**.
//!
//! User accounts are *global* rows: unlike community-scoped state, they have no
//! per-community owner to authorize them, and [`crate::authorize`] otherwise
//! accepts an account from any keyed node that minted its id. That is the right
//! anti-*takeover* rule (a node can only author accounts in its own id namespace),
//! but it does not stop a rogue operator from standing up a node and minting
//! unlimited accounts that replicate everywhere.
//!
//! The trust root closes that: a single [`IssuerRootKeypair`](issuer_root_keypair::IssuerRootKeypair),
//! held offline by the federation operator, signs an [`IssuerCert`](issuer_cert::IssuerCert)
//! for each server permitted to issue accounts. Every node ships the root's public
//! half ([`IssuerRootPublicKey`](issuer_root_public_key::IssuerRootPublicKey)) and,
//! on a global event, requires the minting node to hold a valid cert — rejecting
//! accounts from any un-certified node fleet-wide. A rogue node cannot forge a cert
//! without the root secret.

pub mod choose_issuer;
mod issuer_cert_body;
pub mod issuer_cert;
pub mod issuer_endpoint;
pub mod issuer_root_keypair;
pub mod issuer_root_public_key;
pub mod node_addr_challenge;
