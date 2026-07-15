//! Member-engagement metrics use-cases: a member's per-community contribution
//! and the cached-popularity recompute that gates the franchise. Owns only the
//! content and vote ports it reads, so a metrics consumer doesn't depend on the
//! whole app surface.

use std::collections::HashSet;
use std::sync::Arc;

use domain::{DemosId, PostId, UserId};

use crate::{CommentStore, CommentVoteStore, MembershipStore, PostStore, PostVoteStore, Result};

use super::member_metrics::MemberMetrics;
use super::vote_value::vote_value;

/// Member-engagement metrics use-cases, over just the content and vote stores.
#[derive(Clone)]
pub struct MetricsService {
    posts: Arc<dyn PostStore>,
    comments: Arc<dyn CommentStore>,
    post_votes: Arc<dyn PostVoteStore>,
    comment_votes: Arc<dyn CommentVoteStore>,
    memberships: Arc<dyn MembershipStore>,
}

impl MetricsService {
    pub fn new(
        posts: Arc<dyn PostStore>,
        comments: Arc<dyn CommentStore>,
        post_votes: Arc<dyn PostVoteStore>,
        comment_votes: Arc<dyn CommentVoteStore>,
        memberships: Arc<dyn MembershipStore>,
    ) -> Self {
        Self {
            posts,
            comments,
            post_votes,
            comment_votes,
            memberships,
        }
    }

    /// Compute a member's engagement metrics in one community: net upvotes on
    /// their posts and comments here (plus the counts). Popularity — the sum —
    /// is what gates the franchise and posting policy.
    pub async fn member_metrics(&self, user: UserId, demos: DemosId) -> Result<MemberMetrics> {
        let mut m = MemberMetrics::default();
        for p in self.posts.list_by_author(demos, user).await? {
            if p.removed {
                continue;
            }
            m.posts += 1;
            // Contribution must reflect the *community's* appraisal, not the
            // author's own ballot. Otherwise a user self-qualifies for the
            // franchise (`min_contribution`) and inflates their own
            // `ByContribution` vote weight just by voting on their own content
            // (comments even auto-upvote themselves). Exclude the author's own vote.
            let own = vote_value(self.post_votes.get(p.id, user).await?);
            m.net_post_upvotes += self.post_votes.score(p.id).await? - own;
        }
        // A member's comments in this community = their comments on posts here.
        let here: HashSet<PostId> = self
            .posts
            .list(demos)
            .await?
            .into_iter()
            .map(|p| p.id)
            .collect();
        for c in self.comments.list_by_author(user).await? {
            if c.removed || !here.contains(&c.post_id) {
                continue;
            }
            m.comments += 1;
            let own = vote_value(self.comment_votes.get(c.id, user).await?);
            m.net_comment_upvotes += self.comment_votes.score(c.id).await? - own;
        }
        Ok(m)
    }

    /// Refresh the cached popularity (`Membership::contribution`) for `author` in
    /// `demos` from their current metrics. Called whenever a vote changes the net
    /// score of their content, so eligibility and vote-weighting always read a
    /// current value. A no-op if the author isn't a member.
    pub async fn recompute_popularity(&self, author: UserId, demos: DemosId) -> Result<()> {
        let Some(mut m) = self.memberships.get(author, demos).await? else {
            return Ok(());
        };
        m.contribution = self.member_metrics(author, demos).await?.popularity();
        self.memberships.upsert(m).await
    }
}
