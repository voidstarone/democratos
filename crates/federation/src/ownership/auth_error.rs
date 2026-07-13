//! Why an event failed full authorization.

use crate::FedError;

/// Why an event failed full authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// The signature was absent/malformed or did not verify.
    Fed(FedError),
    /// The producing node has no published key (unknown / not yet joined).
    UnknownNode,
    /// The community has no current owner — nobody is authorized to change it.
    Unowned,
    /// The signer is authentic but is **not** the community's current owner.
    NotOwner,
    /// The event was produced under an epoch older than the current one — the
    /// signer lost ownership (failover) and is fenced out.
    StaleEpoch,
    /// A **global** row (a user account) was signed by a node other than the one
    /// that minted the row's id — i.e. not the account's home node. Closes the
    /// cross-fleet account-takeover hole where any keyed node could overwrite any
    /// user (incl. their password hash).
    WrongHome,
    /// The event's payload does not carry the community it belongs to (a
    /// demos-scoped row with no `demos_id`, a ballot with no parent id, or an
    /// unknown entity). Without a payload-derived community the ownership check
    /// cannot be bound, so the event is refused rather than trusted on its
    /// self-declared `demos`.
    ScopeMismatch,
    /// The event is authentic and authorized, but its row document does not
    /// deserialize into the domain type its entity expects. Applying it would
    /// store a row that every later read then fails to parse — a poisoned-row
    /// denial of service — so it is refused at the boundary instead.
    MalformedPayload,
    /// A **global** row (a user account) was minted by a node that is not a
    /// federation-trusted account issuer — it holds no valid root-signed
    /// [`IssuerCert`](crate::IssuerCert). Only trusted servers may create accounts
    /// that replicate fleet-wide, so an un-certified node's accounts are refused
    /// everywhere. This is what stops a rogue operator standing up a node and
    /// minting accounts across communities.
    UntrustedIssuer,
    /// The control plane could not be consulted.
    Registry(String),
    /// The signer holds the community's ownership lease, but the community's
    /// founder-signed home binding does NOT authorize this node to own it (it is
    /// neither the chosen home node nor a pre-authorized failover heir). This is
    /// what fences out a node that seized the etcd holder key it does not deserve:
    /// it cannot forge the binding without the community's secret key.
    NotBoundHome,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Fed(e) => write!(f, "authenticity: {e}"),
            AuthError::UnknownNode => write!(f, "producing node has no published key"),
            AuthError::Unowned => write!(f, "community has no current owner"),
            AuthError::NotOwner => write!(f, "signer is not the community's owner"),
            AuthError::StaleEpoch => write!(f, "event produced under a stale ownership epoch"),
            AuthError::WrongHome => write!(f, "global row signed by a node that is not its home"),
            AuthError::UntrustedIssuer => {
                write!(f, "account minted by a node that is not a trusted issuer")
            }
            AuthError::ScopeMismatch => write!(f, "event payload carries no resolvable community"),
            AuthError::MalformedPayload => {
                write!(f, "row document does not match its entity's domain type")
            }
            AuthError::Registry(e) => write!(f, "control plane: {e}"),
            AuthError::NotBoundHome => {
                write!(f, "signer is not authorized by the community's home binding")
            }
        }
    }
}
impl std::error::Error for AuthError {}
