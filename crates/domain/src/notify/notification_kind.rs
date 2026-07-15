//! What an in-app notification is about.

use serde::{Deserialize, Serialize};

use crate::{CommentId, DemosId, PostId, TrialId, UserId};

/// What an in-app notification is about. Each variant carries just enough to
/// render a one-line summary and link straight to the source. The two kinds map
/// to the two triggers a member can opt into: being named in content, and being
/// summoned to a jury.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum NotificationKind {
    /// The recipient's handle was written as `@handle` in a post or comment.
    /// `comment` is `Some` when the mention was in a reply (the link still points
    /// at the post, anchored to the comment); `None` for a mention in the post
    /// body itself. `by` is the author who wrote it.
    Mention {
        post: PostId,
        comment: Option<CommentId>,
        by: UserId,
    },
    /// The recipient was empanelled on a jury and must return a verdict.
    JurySummons { trial: TrialId, demos: DemosId },
    /// A new comment was posted on a trial the recipient is party to (accused,
    /// reporter, juror) or has already spoken in. `by` is the commenter.
    TrialComment {
        trial: TrialId,
        demos: DemosId,
        by: UserId,
    },
}
