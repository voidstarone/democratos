//! A case against a target.

use serde::{Deserialize, Serialize};

use crate::{DemosId, Flag, ReportId, ReportReason, ReportStatus, ReportTarget, Timestamp, UserId};

/// A case against a target. It is opened by a founding flag and gathers further
/// flags while it stays [`ReportStatus::Open`]; `flags` is therefore always
/// non-empty.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Report {
    pub id: ReportId,
    pub demos_id: DemosId,
    pub target: ReportTarget,
    pub flags: Vec<Flag>,
    /// When the case was opened (the founding flag's time).
    pub created_at: Timestamp,
    pub status: ReportStatus,
    /// Optimistic-concurrency revision — see [`crate::Proposal::rev`]. Guards the
    /// read-modify-write on a report (merging flags, opening a trial) against a
    /// concurrent update on another replica silently overwriting it.
    #[serde(default)]
    pub rev: u64,
}

impl Report {
    pub fn new(
        id: ReportId,
        demos_id: DemosId,
        reporter: Option<UserId>,
        target: ReportTarget,
        reason: ReportReason,
        note: impl Into<String>,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            demos_id,
            target,
            flags: vec![Flag {
                reporter,
                reason,
                note: note.into(),
                created_at,
            }],
            created_at,
            status: ReportStatus::Open,
            rev: 0,
        }
    }

    /// The flag that opened the case. Never panics: `flags` is non-empty by
    /// construction.
    pub fn founding(&self) -> &Flag {
        &self.flags[0]
    }

    /// Fold another accusation into this case. A flag identical in reporter and
    /// reason to one already present is ignored — a detector re-running, or a
    /// member re-submitting, should not inflate the charge sheet. Returns
    /// whether the flag was added.
    pub fn add_flag(
        &mut self,
        reporter: Option<UserId>,
        reason: ReportReason,
        note: impl Into<String>,
        created_at: Timestamp,
    ) -> bool {
        if self
            .flags
            .iter()
            .any(|f| f.reporter == reporter && f.reason == reason)
        {
            return false;
        }
        self.flags.push(Flag {
            reporter,
            reason,
            note: note.into(),
            created_at,
        });
        true
    }

    /// True when every flag on the case was filed automatically (no member has
    /// weighed in).
    pub fn is_automatic(&self) -> bool {
        self.flags.iter().all(Flag::is_automatic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PostId, RuleId};

    fn post_report() -> Report {
        Report::new(
            ReportId(1),
            DemosId(1),
            Some(UserId(7)),
            ReportTarget::Post(PostId(42)),
            ReportReason::RuleBreak { rule: None },
            "breaks rule 1",
            Timestamp(0),
        )
    }

    #[test]
    fn a_new_report_has_exactly_its_founding_flag() {
        let r = post_report();
        assert_eq!(r.flags.len(), 1);
        assert_eq!(r.founding().reason, ReportReason::RuleBreak { rule: None });
        assert!(!r.is_automatic(), "filed by a member");
    }

    #[test]
    fn a_different_reason_is_folded_in() {
        let mut r = post_report();
        let added = r.add_flag(None, ReportReason::Nsfw, "auto: NSFW", Timestamp(10));
        assert!(added);
        assert_eq!(r.flags.len(), 2);
    }

    #[test]
    fn an_identical_flag_is_not_duplicated() {
        let mut r = post_report();
        // Same reporter + reason as the founding flag.
        let added = r.add_flag(
            Some(UserId(7)),
            ReportReason::RuleBreak { rule: None },
            "again",
            Timestamp(10),
        );
        assert!(!added);
        assert_eq!(r.flags.len(), 1);
    }

    #[test]
    fn distinct_rule_breaks_are_distinct_flags() {
        let mut r = post_report();
        assert!(r.add_flag(
            Some(UserId(8)),
            ReportReason::RuleBreak {
                rule: Some(RuleId(2))
            },
            "breaks rule 2",
            Timestamp(10),
        ));
        assert_eq!(r.flags.len(), 2);
    }
}
