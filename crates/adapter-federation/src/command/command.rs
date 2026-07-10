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
}
