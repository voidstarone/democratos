//! Where a report is in its lifecycle.

use serde::{Deserialize, Serialize};

use crate::TrialId;

/// Where a report is in its lifecycle.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ReportStatus {
    Open,
    Dismissed,
    /// A jury trial has been empanelled.
    OnTrial(TrialId),
    /// Trial concluded against the accused.
    Upheld,
}
