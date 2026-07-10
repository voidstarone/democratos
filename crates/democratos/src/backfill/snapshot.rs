//! The whole on-disk dataset.

use serde::Deserialize;

use domain::{Comment, Demos, Membership, Post, Proposal, Report, Rule, Trial, User};

use crate::backfill::jury_ballot_rec::JuryBallotRec;
use crate::backfill::post_vote_rec::PostVoteRec;
use crate::backfill::vote_rec::VoteRec;

/// The whole on-disk dataset. `#[serde(default)]` so a snapshot missing a
/// (newer) collection still loads.
#[derive(Deserialize, Default)]
pub(crate) struct Snapshot {
    #[serde(default)]
    pub(crate) users: Vec<User>,
    #[serde(default)]
    pub(crate) demoi: Vec<Demos>,
    #[serde(default)]
    pub(crate) memberships: Vec<Membership>,
    #[serde(default)]
    pub(crate) proposals: Vec<Proposal>,
    #[serde(default)]
    pub(crate) votes: Vec<VoteRec>,
    #[serde(default)]
    pub(crate) post_votes: Vec<PostVoteRec>,
    #[serde(default)]
    pub(crate) rules: Vec<Rule>,
    #[serde(default)]
    pub(crate) posts: Vec<Post>,
    #[serde(default)]
    pub(crate) comments: Vec<Comment>,
    #[serde(default)]
    pub(crate) reports: Vec<Report>,
    #[serde(default)]
    pub(crate) trials: Vec<Trial>,
    #[serde(default)]
    pub(crate) jury_ballots: Vec<JuryBallotRec>,
}
