//! One reviewer's classification of a flagged item.

use serde::{Deserialize, Serialize};

use crate::sensitive::sensitive_tag::SensitiveTag;
use crate::{Timestamp, UserId};

/// One reviewer's classification vote on a [`SensitiveCase`](crate::SensitiveCase).
/// A reviewer holds at most one vote per case (a re-vote replaces it).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ReviewVote {
    pub reviewer: UserId,
    pub tag: SensitiveTag,
    pub at: Timestamp,
}
