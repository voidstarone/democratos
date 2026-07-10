//! A single accusation folded into a report.

use serde::{Deserialize, Serialize};

use crate::{ReportReason, Timestamp, UserId};

/// A single accusation folded into a report: who raised it, why, when, and the
/// note they left. One report on a target accumulates many flags — a post
/// flagged again for a different reason adds a flag to the existing case rather
/// than opening a parallel report.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Flag {
    /// `None` means filed automatically by a detector — "the machine accuses,
    /// the demos judges".
    pub reporter: Option<UserId>,
    pub reason: ReportReason,
    pub note: String,
    pub created_at: Timestamp,
}

impl Flag {
    pub fn is_automatic(&self) -> bool {
        self.reporter.is_none()
    }
}
