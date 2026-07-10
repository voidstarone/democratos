//! The governance **write** operations whose authoritative home is a
//! community's owner node.

use async_trait::async_trait;

use domain::{PostId, ProposalId, TrialId, UserId, Verdict};

use crate::{CastJuryVoteError, CastVoteError, Result, VotePostError};

/// The governance **write** operations whose authoritative home is a community's
/// owner node. A delivery adapter (web/CLI) submits these through this port
/// rather than calling [`Services`](crate::Services) directly, so the
/// composition root can plug in either a single-box implementation that runs the
/// use-case locally ([`LocalWrites`](crate::LocalWrites)) or a federated one that
/// routes the write to the owner and replicates it. Reads stay on `Services`;
/// only these correctness-critical writes need routing.
#[async_trait]
pub trait GovernanceWrites: Send + Sync {
    /// Cast a governance ballot on a proposal. `sig` is the acting user's Ed25519
    /// signature over the canonical vote message; it travels with the intent so the
    /// community's **owner** can verify it (never trusting a forwarding node's word
    /// for who voted). `None` is only accepted for accounts with no enrolled key.
    async fn cast_vote(
        &self,
        proposal: ProposalId,
        voter: UserId,
        aye: bool,
        sig: Option<String>,
    ) -> Result<(), CastVoteError>;
    /// Up/down/clear a post vote; returns the post's new net score. `sig` is the
    /// acting user's signature over the canonical post-vote message (the resolved
    /// direction), verified by the owner so a relay can't forge it.
    async fn vote_post(
        &self,
        post: PostId,
        user: UserId,
        dir: Option<bool>,
        sig: Option<String>,
    ) -> Result<i64, VotePostError>;
    /// Cast a juror's ballot in a trial; returns the trial's verdict after it.
    /// `sig` is the juror's signature over the canonical jury-vote message.
    async fn cast_jury_vote(
        &self,
        trial: TrialId,
        juror: UserId,
        guilty: bool,
        sig: Option<String>,
    ) -> Result<Verdict, CastJuryVoteError>;
}
