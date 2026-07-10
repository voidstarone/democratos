//! Why something was reported.

use serde::{Deserialize, Serialize};

use crate::RuleId;

/// Why something was reported.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ReportReason {
    /// Suspected automated account. Filed by the detector or a member.
    Bot,
    /// Breaks a community rule (optionally a specific one).
    RuleBreak { rule: Option<RuleId> },
    /// NSFW content in a community that has voted to forbid it. Filed by the
    /// NSFW detector ("the machine flags; the demos judges") or a member.
    Nsfw,
}
