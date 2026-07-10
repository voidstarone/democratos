//! The mutex-guarded in-memory dataset behind [`MemoryStore`](crate::MemoryStore).

use std::collections::HashMap;

use domain::{
    Comment, Demos, FoundingPetition, Membership, Post, Proposal, Report, Rule, Trial, User,
};

use crate::comment_vote_rec::CommentVoteRec;
use crate::jury_ballot_rec::JuryBallotRec;
use crate::post_vote_rec::PostVoteRec;
use crate::vote_rec::VoteRec;

#[derive(Default)]
pub(crate) struct Inner {
    pub(crate) users: Vec<User>,
    pub(crate) demoi: Vec<Demos>,
    pub(crate) foundings: Vec<FoundingPetition>,
    pub(crate) memberships: Vec<Membership>,
    pub(crate) proposals: Vec<Proposal>,
    pub(crate) votes: Vec<VoteRec>,
    pub(crate) post_votes: Vec<PostVoteRec>,
    pub(crate) comment_votes: Vec<CommentVoteRec>,
    pub(crate) rules: Vec<Rule>,
    pub(crate) posts: Vec<Post>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) reports: Vec<Report>,
    pub(crate) trials: Vec<Trial>,
    pub(crate) jury_ballots: Vec<JuryBallotRec>,
    /// Uploaded media bytes, keyed by storage key → (content-type, bytes).
    pub(crate) media: HashMap<String, (String, Vec<u8>)>,
    pub(crate) next_user: u64,
    pub(crate) next_demos: u64,
    pub(crate) next_founding: u64,
    pub(crate) next_proposal: u64,
    pub(crate) next_rule: u64,
    pub(crate) next_post: u64,
    pub(crate) next_comment: u64,
    pub(crate) next_report: u64,
    pub(crate) next_trial: u64,
}
