//! The entire persisted dataset, as one serializable value.

use serde::{Deserialize, Serialize};

use domain::{
    Comment, Demos, FoundingPetition, InviteRequest, Membership, Notification, Post, Proposal,
    Report, Rule, SensitiveCase, Trial, TrialComment, User,
};

use crate::comment_vote_rec::CommentVoteRec;
use crate::jury_ballot_rec::JuryBallotRec;
use crate::post_vote_rec::PostVoteRec;
use crate::vote_rec::VoteRec;

/// The entire persisted dataset.
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Db {
    pub(crate) users: Vec<User>,
    pub(crate) demoi: Vec<Demos>,
    #[serde(default)]
    pub(crate) foundings: Vec<FoundingPetition>,
    pub(crate) memberships: Vec<Membership>,
    pub(crate) proposals: Vec<Proposal>,
    pub(crate) votes: Vec<VoteRec>,
    #[serde(default)]
    pub(crate) post_votes: Vec<PostVoteRec>,
    #[serde(default)]
    pub(crate) comment_votes: Vec<CommentVoteRec>,
    pub(crate) rules: Vec<Rule>,
    pub(crate) posts: Vec<Post>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) reports: Vec<Report>,
    #[serde(default)]
    pub(crate) invites: Vec<InviteRequest>,
    #[serde(default)]
    pub(crate) sensitive_cases: Vec<SensitiveCase>,
    pub(crate) trials: Vec<Trial>,
    #[serde(default)]
    pub(crate) trial_comments: Vec<TrialComment>,
    pub(crate) jury_ballots: Vec<JuryBallotRec>,
    #[serde(default)]
    pub(crate) notifications: Vec<Notification>,
    pub(crate) next_user: u64,
    pub(crate) next_demos: u64,
    #[serde(default)]
    pub(crate) next_founding: u64,
    pub(crate) next_proposal: u64,
    pub(crate) next_rule: u64,
    pub(crate) next_post: u64,
    pub(crate) next_comment: u64,
    pub(crate) next_report: u64,
    #[serde(default)]
    pub(crate) next_invite: u64,
    #[serde(default)]
    pub(crate) next_sensitive_case: u64,
    pub(crate) next_trial: u64,
    #[serde(default)]
    pub(crate) next_trial_comment: u64,
    #[serde(default)]
    pub(crate) next_notification: u64,
    /// The persisted invitation-only toggle; `None` until the operator sets it.
    #[serde(default)]
    pub(crate) invite_only: Option<bool>,
}
