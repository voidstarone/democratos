//! Why minting an account (locally or via a trusted issuer) failed.

use thiserror::Error;

/// Why minting a real account failed. A delivery adapter surfaces `Rejected` to the
/// user as a 4xx (bad/duplicate handle or email, weak password), and the other two
/// as a "try again later" 5xx — the sign-up was NOT completed.
#[derive(Debug, Error)]
pub enum MintAccountError {
    /// No trusted issuer was reachable to mint the account. This node is not itself
    /// a trusted issuer and could not find/reach one, so it fails closed rather than
    /// mint an account that would be rejected fleet-wide.
    #[error("no trusted issuer is currently available to create the account")]
    NoIssuerAvailable,

    /// A trusted issuer (or the local registration) rejected the sign-up on its
    /// merits — a taken handle/email, or a password that fails the policy.
    #[error("{0}")]
    Rejected(String),

    /// An infrastructure failure reaching or running the mint. The account was not
    /// created; the user can retry.
    #[error("account creation is temporarily unavailable: {0}")]
    Unavailable(String),
}
