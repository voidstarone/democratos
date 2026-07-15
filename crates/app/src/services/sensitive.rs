//! Facade delegators for platform-wide sensitive-content review use-cases. The
//! logic now lives in
//! [`SensitiveReviewService`](super::sensitive_review_service::SensitiveReviewService);
//! these thin methods keep `services.flag_sensitive()` and friends working for
//! call sites not yet migrated off the `Services` aggregator.

use domain::{Post, ReportTarget, SensitiveCase, SensitiveCaseId, SensitiveTag, UserId};
use domain::Visibility;

use crate::{Result, SensitiveReviewError};

use super::sensitive_review_service::SensitiveReviewService;
use super::services::Services;

impl Services {
    /// Build the extracted [`SensitiveReviewService`] from the ports this aggregator
    /// still holds. Cheap — `Arc` clones only — so delegators construct one per call
    /// rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `SensitiveReviewService`
    /// directly.
    pub(super) fn sensitive_review_service(&self) -> SensitiveReviewService {
        SensitiveReviewService::new(
            self.sensitive_cases.clone(),
            self.users.clone(),
            self.posts.clone(),
            self.comments.clone(),
            self.requires_age_verification,
            self.clock.clone(),
        )
    }

    /// Opt an account in to (or out of) reviewing platform-wide sensitive content.
    /// Default off; deliberately not a demos tier.
    pub async fn set_sensitive_reviewer(
        &self,
        user: UserId,
        is_reviewer: bool,
    ) -> Result<(), SensitiveReviewError> {
        self.sensitive_review_service()
            .set_sensitive_reviewer(user, is_reviewer)
            .await
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
        self.sensitive_review_service()
            .flag_sensitive(reporter, target, note)
            .await
    }

    /// The open review queue — reviewer-only.
    pub async fn list_review_queue(
        &self,
        reviewer: UserId,
    ) -> Result<Vec<SensitiveCase>, SensitiveReviewError> {
        self.sensitive_review_service()
            .list_review_queue(reviewer)
            .await
    }

    /// How many cases are open — backs the reviewer nav badge.
    pub async fn open_case_count(&self) -> Result<u64> {
        self.sensitive_review_service().open_case_count().await
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
        self.sensitive_review_service()
            .cast_review_vote(reviewer, case_id, tag)
            .await
    }

    /// How `post` should be presented to `viewer` under the deployment's age
    /// policy — the one place the gate is decided ([`domain::visibility`]).
    pub async fn post_visibility(&self, post: &Post, viewer: UserId) -> Result<Visibility> {
        self.sensitive_review_service()
            .post_visibility(post, viewer)
            .await
    }
}
