use serde::{Deserialize, Serialize};

use domain::Verdict;

/// The result of executing a [`Command`](crate::Command) on the owner.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CommandOutcome {
    Voted,
    /// The post's new net score.
    PostScore(i64),
    /// The trial's verdict after the ballot.
    Verdict(Verdict),
}
