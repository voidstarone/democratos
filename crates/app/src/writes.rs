//! The single-box implementation of the [`GovernanceWrites`] port: run every
//! write against the local [`Services`] directly.
//!
//! In an un-federated deployment there is exactly one node, which owns every
//! community, so a "route the write to the owner" gateway would just call the
//! local use-case anyway. [`LocalWrites`] *is* that call — it lets the web/CLI
//! adapters depend only on the port while the composition root decides whether
//! writes go straight to `Services` (here) or through the federation router.

use async_trait::async_trait;

use domain::{PostId, ProposalId, TrialId, UserId, Verdict};

use crate::GovernanceWrites;
use crate::Services;
use crate::{CastJuryVoteError, CastVoteError, Result, VotePostError};

/// Runs governance writes against the local `Services` — correct whenever this
/// process owns the target community (always true single-box).
pub struct LocalWrites {
    services: Services,
}

impl LocalWrites {
    pub fn new(services: Services) -> Self {
        Self { services }
    }
}

#[async_trait]
impl GovernanceWrites for LocalWrites {
    async fn cast_vote(
        &self,
        proposal: ProposalId,
        voter: UserId,
        aye: bool,
        sig: Option<String>,
    ) -> Result<(), CastVoteError> {
        self.services
            .cast_vote(proposal, voter, aye, sig.as_deref())
            .await
    }

    async fn vote_post(
        &self,
        post: PostId,
        user: UserId,
        dir: Option<bool>,
        sig: Option<String>,
    ) -> Result<i64, VotePostError> {
        self.services
            .vote_post(post, user, dir, sig.as_deref())
            .await
    }

    async fn cast_jury_vote(
        &self,
        trial: TrialId,
        juror: UserId,
        guilty: bool,
        sig: Option<String>,
    ) -> Result<Verdict, CastJuryVoteError> {
        self.services
            .cast_jury_vote(trial, juror, guilty, sig.as_deref())
            .await
    }
}
