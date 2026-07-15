//! Persistence for comments on trials (the community gallery discussion).

use async_trait::async_trait;

use domain::{TrialComment, TrialId, UserId};
use domain::Timestamp;

use crate::Result;

#[async_trait]
pub trait TrialCommentStore: Send + Sync {
    /// Record a comment on `trial` by `author` and return it (id assigned by the
    /// store). The caller has already checked the author may comment.
    async fn add(
        &self,
        trial: TrialId,
        author: UserId,
        body: String,
        at: Timestamp,
    ) -> Result<TrialComment>;
    /// Every comment on `trial`, oldest first — a trial's discussion reads top to
    /// bottom like any thread. Trials are public record, so is their gallery.
    async fn list_for_trial(&self, trial: TrialId) -> Result<Vec<TrialComment>>;
}
