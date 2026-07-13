//! The `issuer` subcommand set — managing federation-trusted account issuers.

use clap::Subcommand;

/// Manage **trusted account issuers**: the servers whose user accounts are honoured
/// across every community. Trust is rooted in a single federation keypair held
/// offline; these commands generate it, sign per-node certificates with it, and
/// publish those certs to the control plane. See `federation::issuer`.
#[derive(Subcommand)]
pub(crate) enum IssuerCommand {
    /// Generate a fresh federation trust-root keypair. Prints the SECRET seed (store
    /// it OFFLINE — anyone holding it can certify a rogue node) and the PUBLIC key to
    /// set as `FEDERATION_TRUST_ROOT` on every node. Run once, when establishing a
    /// federation.
    Root,

    /// Sign a trusted-issuer certificate for a node, using the root secret. Offline:
    /// run it on the machine that holds the root seed, not on a node. Prints the cert
    /// as JSON to hand to `issuer publish`.
    Certify {
        /// The node id to certify as a trusted account issuer.
        #[arg(long)]
        node: u16,

        /// Cert epoch. A higher epoch supersedes a lower one — bump it to rotate an
        /// issuer's grant. Defaults to 1.
        #[arg(long, default_value_t = 1)]
        epoch: u64,

        /// The federation root's 32-byte secret seed (hex). Prefer the env var so it
        /// never lands in shell history.
        #[arg(long, env = "FEDERATION_ROOT_SEED")]
        root_seed: String,
    },

    /// Publish a signed issuer cert (the JSON from `certify`) into the control plane,
    /// so peers honour that node's accounts fleet-wide. Run on a node with etcd
    /// access; `FEDERATION_TRUST_ROOT` must be set so the cert is verified before it
    /// is stored.
    Publish {
        /// The cert JSON produced by `certify` (inline). Mutually exclusive with
        /// `--cert-file`.
        #[arg(long)]
        cert: Option<String>,

        /// Path to a file holding the cert JSON. Mutually exclusive with `--cert`.
        #[arg(long)]
        cert_file: Option<String>,

        /// Control-plane etcd endpoints (comma-separated).
        #[arg(long, env = "DEMOCRATOS_ETCD_ENDPOINTS")]
        etcd_endpoints: String,
    },
}
