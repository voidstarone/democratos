

use domain::{
    outcome_for, Post, ReportTarget, ReviewOutcome, SensitiveCase, SensitiveCaseId,
    SensitiveTag, UserId,
};

use domain::{visibility, Visibility};

use crate::{
    Result, SensitiveReviewError, StoreError,
};


use super::escalate_to_operator::escalate_to_operator;
use super::services::Services;

impl Services {
    /// Opt an account in to (or out of) reviewing platform-wide sensitive content.
    /// Default off; deliberately not a demos tier.
    pub async fn set_sensitive_reviewer(
        &self,
        user: UserId,
        is_reviewer: bool,
    ) -> Result<(), SensitiveReviewError> {
        Ok(self.users.set_sensitive_reviewer(user, is_reviewer).await?)
    }

    /// Flag a post/comment as sensitive. Any signed-in user may flag; the content
    /// is **hidden pending review immediately** and a platform-wide review case is
    /// opened (or the flag merges into the open one). Returns the case.
    pub async fn flag_sensitive(
        &self,
        reporter: UserId,
        target: ReportTarget,
        note: &str,
    ) -> Result<SensitiveCase, SensitiveReviewError> {
        // Hide the target now — flagging errs toward caution.
        match target {
            ReportTarget::Post(p) => {
                if self.posts.get(p).await?.is_none() {
                    return Err(SensitiveReviewError::Rejected("no such post".into()));
                }
                self.posts.set_pending_review(p, true).await?;
            }
            ReportTarget::Comment(c) => {
                if self.comments.get(c).await?.is_none() {
                    return Err(SensitiveReviewError::Rejected("no such comment".into()));
                }
                self.comments.set_pending_review(c, true).await?;
            }
            ReportTarget::User(_) => {
                return Err(SensitiveReviewError::Rejected(
                    "only posts and comments can be flagged sensitive".into(),
                ))
            }
        }
        let now = self.clock.now();
        match self.sensitive_cases.open_for_target(target).await? {
            Some(case) => Ok(case),
            None => Ok(self
                .sensitive_cases
                .create(Some(reporter), target, note, now)
                .await?),
        }
    }

    /// The open review queue — reviewer-only.
    pub async fn list_review_queue(
        &self,
        reviewer: UserId,
    ) -> Result<Vec<SensitiveCase>, SensitiveReviewError> {
        self.require_sensitive_reviewer(reviewer).await?;
        Ok(self.sensitive_cases.list_open().await?)
    }

    /// How many cases are open — backs the reviewer nav badge.
    pub async fn open_case_count(&self) -> Result<u64> {
        self.sensitive_cases.count_open().await
    }

    /// Cast a reviewer's classification on a case. Reviewer-only; one vote per
    /// reviewer (a repeat corrects it). Once at least
    /// [`REVIEW_QUORUM`](domain::REVIEW_QUORUM) reviewers have voted, the plurality
    /// tag resolves the case and its disposition is applied to the content.
    pub async fn cast_review_vote(
        &self,
        reviewer: UserId,
        case_id: SensitiveCaseId,
        tag: SensitiveTag,
    ) -> Result<SensitiveCase, SensitiveReviewError> {
        self.require_sensitive_reviewer(reviewer).await?;
        let mut case = self
            .sensitive_cases
            .get(case_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        if case.status != domain::SensitiveCaseStatus::Open {
            return Err(SensitiveReviewError::AlreadyResolved);
        }
        let now = self.clock.now();
        case.cast(reviewer, tag, now);
        // Resolve if the quorum is now met, and carry out the disposition.
        if let Some(winner) = case.try_resolve() {
            self.apply_review_outcome(case.target, outcome_for(winner)).await?;
        }
        self.sensitive_cases.update(&case).await?;
        Ok(case)
    }

    /// Carry out a resolved case's disposition on its target.
    async fn apply_review_outcome(
        &self,
        target: ReportTarget,
        outcome: ReviewOutcome,
    ) -> Result<(), SensitiveReviewError> {
        match (target, outcome) {
            // False flag → un-hide, unchanged.
            (ReportTarget::Post(p), ReviewOutcome::Restore) => {
                self.posts.set_pending_review(p, false).await?;
            }
            (ReportTarget::Comment(c), ReviewOutcome::Restore) => {
                self.comments.set_pending_review(c, false).await?;
            }
            // Lawful adult content → un-hide but NSFW-gate (posts carry the flag;
            // comments have no NSFW blur, so they are simply restored).
            (ReportTarget::Post(p), ReviewOutcome::AgeGate) => {
                self.posts.set_is_nsfw(p, true).await?;
                self.posts.set_pending_review(p, false).await?;
            }
            (ReportTarget::Comment(c), ReviewOutcome::AgeGate) => {
                self.comments.set_pending_review(c, false).await?;
            }
            // Upheld → take down platform-wide.
            (ReportTarget::Post(p), ReviewOutcome::Remove { escalate }) => {
                self.posts.set_removed(p, true).await?;
                self.posts.set_pending_review(p, false).await?;
                if escalate {
                    escalate_to_operator(target);
                }
            }
            (ReportTarget::Comment(c), ReviewOutcome::Remove { escalate }) => {
                self.comments.set_removed(c, true).await?;
                self.comments.set_pending_review(c, false).await?;
                if escalate {
                    escalate_to_operator(target);
                }
            }
            (ReportTarget::User(_), _) => {}
        }
        Ok(())
    }

    /// The reviewer gate: the account must have opted in to sensitive-content
    /// review. Deliberately a platform account attribute, not a demos membership.
    async fn require_sensitive_reviewer(
        &self,
        user: UserId,
    ) -> Result<(), SensitiveReviewError> {
        let u = self
            .users
            .get(user)
            .await?
            .ok_or(StoreError::NotFound)?;
        if !u.is_sensitive_reviewer {
            return Err(SensitiveReviewError::NotReviewer);
        }
        Ok(())
    }

    /// Run age verification for `user` through the provider; on success persist
    /// the result. Returns whether the user is now verified.
    pub async fn verify_age(&self, user: UserId) -> Result<bool> {
        let is_verified = self.age_verifier.verify(user).await?;
        if is_verified {
            self.users.set_is_age_verified(user, true).await?;
        }
        Ok(is_verified)
    }

    /// How `post` should be presented to `viewer` under the deployment's age
    /// policy — the one place the gate is decided ([`domain::visibility`]).
    pub async fn post_visibility(&self, post: &Post, viewer: UserId) -> Result<Visibility> {
        let is_viewer_age_verified = self
            .users
            .get(viewer)
            .await?
            .map_or(false, |u| u.is_age_verified);
        Ok(visibility(
            post.is_nsfw,
            is_viewer_age_verified,
            self.requires_age_verification,
        ))
    }
}
