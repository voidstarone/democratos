use async_trait::async_trait;

use app::{CastJuryVoteError, CastVoteError, StoreError, VotePostError};
use domain::{PostId, ProposalId, TrialId, UserId, Verdict};

use crate::{Command, CommandOutcome, ForwardError, WriteRouter};

/// A routing failure that is not an owner-side store outcome nor a domain
/// rejection (i.e. the write never reached an owner, or none was reachable) is a
/// transport failure — surfaced verbatim as a store failure through the port.
fn transport_failure(message: String) -> StoreError {
    StoreError::Store(message)
}

/// The federated implementation of [`app::GovernanceWrites`]: every governance
/// write is routed to its community's owner (running locally with quorum-of-2
/// durability when this node owns it), so a vote entered on any node is recorded
/// authoritatively on exactly one. This is what the composition root hands the
/// web/CLI adapters when federation is enabled; single-box uses
/// [`app::LocalWrites`] instead.
pub struct FederatedWrites {
    router: WriteRouter,
}

impl FederatedWrites {
    pub fn new(router: WriteRouter) -> Self {
        Self { router }
    }
}

#[async_trait]
impl app::GovernanceWrites for FederatedWrites {
    async fn cast_vote(
        &self,
        proposal: ProposalId,
        voter: UserId,
        aye: bool,
        sig: Option<String>,
    ) -> Result<(), CastVoteError> {
        self.router
            .submit(Command::CastVote {
                proposal: proposal.0,
                voter: voter.0,
                aye,
                sig,
            })
            .await
            .map(|_| ())
            // A domain rejection keeps its human-readable message as `Rejected`; an
            // owner-side store outcome keeps its typed `StoreError` (so e.g. a double
            // vote still surfaces as `AlreadyVoted`); a transport failure is a store
            // failure.
            .map_err(|e| match e {
                ForwardError::App(store) => CastVoteError::Store(store),
                ForwardError::Rejected(s) => CastVoteError::Rejected(s),
                other => CastVoteError::Store(transport_failure(other.to_string())),
            })
    }

    async fn vote_post(
        &self,
        post: PostId,
        user: UserId,
        dir: Option<bool>,
        sig: Option<String>,
    ) -> Result<i64, VotePostError> {
        match self
            .router
            .submit(Command::VotePost {
                post: post.0,
                user: user.0,
                dir,
                sig,
            })
            .await
            .map_err(|e| match e {
                ForwardError::App(store) => VotePostError::Store(store),
                ForwardError::Rejected(s) => VotePostError::Rejected(s),
                other => VotePostError::Store(transport_failure(other.to_string())),
            })? {
            CommandOutcome::PostScore(score) => Ok(score),
            other => Err(VotePostError::Store(transport_failure(format!(
                "unexpected outcome for vote_post: {other:?}"
            )))),
        }
    }

    async fn cast_jury_vote(
        &self,
        trial: TrialId,
        juror: UserId,
        guilty: bool,
        sig: Option<String>,
    ) -> Result<Verdict, CastJuryVoteError> {
        match self
            .router
            .submit(Command::CastJuryVote {
                trial: trial.0,
                juror: juror.0,
                guilty,
                sig,
            })
            .await
            .map_err(|e| match e {
                ForwardError::App(store) => CastJuryVoteError::Store(store),
                ForwardError::Rejected(s) => CastJuryVoteError::Rejected(s),
                other => CastJuryVoteError::Store(transport_failure(other.to_string())),
            })? {
            CommandOutcome::Verdict(v) => Ok(v),
            other => Err(CastJuryVoteError::Store(transport_failure(format!(
                "unexpected outcome for cast_jury_vote: {other:?}"
            )))),
        }
    }
}
