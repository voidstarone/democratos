//! A comment on a trial — the community's discussion of a live or settled case.

use serde::{Deserialize, Serialize};

use crate::{Timestamp, TrialCommentId, TrialId, UserId};

/// A single comment on a trial. Any enfranchised voter of the trial's demos may
/// post one — the gallery is open to the electorate, whether or not they sit on
/// the jury — so that a case is argued in the open, not just decided behind the
/// panel. Jurors weigh the argument but are not bound by it. Like the trial
/// itself, comments are public record.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TrialComment {
    pub id: TrialCommentId,
    pub trial_id: TrialId,
    pub author: UserId,
    pub body: String,
    pub created_at: Timestamp,
}

impl TrialComment {
    pub fn new(
        id: TrialCommentId,
        trial_id: TrialId,
        author: UserId,
        body: impl Into<String>,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            trial_id,
            author,
            body: body.into(),
            created_at,
        }
    }
}
