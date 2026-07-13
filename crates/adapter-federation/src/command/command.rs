use serde::{Deserialize, Serialize};

/// A forwardable write intent. Extend as more write use-cases become federated;
/// votes are here because they are the correctness-critical ones.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Command {
    /// Cast a governance ballot on a proposal. `sig` is the voter's signature over
    /// the canonical vote message, carried so the owner re-verifies who voted
    /// rather than trusting this forwarding node.
    CastVote {
        proposal: u64,
        voter: u64,
        aye: bool,
        sig: Option<String>,
    },
    /// Up/down/clear a post vote (`dir`: Some(true)=up, Some(false)=down, None=clear).
    /// `sig` is the voter's signature over the canonical post-vote message.
    VotePost {
        post: u64,
        user: u64,
        dir: Option<bool>,
        sig: Option<String>,
    },
    /// Cast a juror's ballot in a trial. `sig` is the juror's signature over the
    /// canonical jury-vote message, re-verified by the owner.
    CastJuryVote {
        trial: u64,
        juror: u64,
        guilty: bool,
        sig: Option<String>,
    },
    /// Ask a **trusted issuer** to mint a new account on the requesting node's
    /// behalf. A node that is not itself a trusted issuer cannot create an account
    /// that replicates fleet-wide, so it forwards the sign-up here; the issuer runs
    /// its normal registration (validating the credential policy and hashing the
    /// password itself) and mints the account in its own id namespace. The raw
    /// password travels only over the authenticated, TLS-protected node link and is
    /// never persisted by the forwarder.
    MintAccount {
        handle: String,
        email: String,
        password: String,
    },
    /// Ask an account's **home** trusted issuer to verify a login on the requesting
    /// node's behalf. Credentials (email + hash) never replicate, so a community node
    /// can't verify a federated account's password itself; it forwards the handle +
    /// password to the issuer that homes the account, which verifies and returns the
    /// account id. Login is by handle because handles replicate (emails do not).
    Authenticate { handle: String, password: String },
}
