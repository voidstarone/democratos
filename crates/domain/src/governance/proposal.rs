//! A governance proposal and its close/apply lifecycle.

use serde::{Deserialize, Serialize};

use crate::{
    decide, threshold_for, DecisionClass, DemosId, Phase, ProposalId, ProposalKind, ProposalStatus,
    Tally, Timestamp, UserId, RECALL_WINDOW_DAYS,
};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalId,
    pub demos_id: DemosId,
    pub proposer: UserId,
    pub kind: ProposalKind,
    pub opened_at: Timestamp,
    pub closes_at: Timestamp,
    pub status: ProposalStatus,
    /// Whether the effects of a *passed* proposal (a rule/criteria/policy change)
    /// have already been applied to the demos. The application applies them once,
    /// after the timelock matures, and sets this — so re-invoking the close/apply
    /// path is idempotent and can't, e.g., add the same rule twice or reset the
    /// timelock. `#[serde(default)]` reads older records as `false` (not yet
    /// applied), which is the safe default. Meaningless while `status` is `Open`.
    #[serde(default)]
    pub applied: bool,
    /// Optimistic-concurrency revision. Bumped by the store on every persisted
    /// update; a write that reads revision *r* only lands if the row is still at
    /// *r*, so two replicas doing a read-modify-write can't silently clobber each
    /// other (a lost update). `#[serde(default)]` reads older rows as `0`.
    #[serde(default)]
    pub rev: u64,
}

impl Proposal {
    pub fn new(
        id: ProposalId,
        demos_id: DemosId,
        proposer: UserId,
        kind: ProposalKind,
        opened_at: Timestamp,
        closes_at: Timestamp,
    ) -> Self {
        Self {
            id,
            demos_id,
            proposer,
            kind,
            opened_at,
            closes_at,
            status: ProposalStatus::Open,
            applied: false,
            rev: 0,
        }
    }

    /// Layers 3 & 4 together — close the proposal at `closed_at`: apply the
    /// phase-appropriate threshold, and on a passing constitutional change apply
    /// the timelock so the result only becomes effective after the recall window.
    ///
    /// `effective_at` is measured from the actual close moment (`closed_at`), not
    /// from the voting deadline — so a proposal closed early takes effect
    /// immediately (or, if constitutional, a recall window later).
    ///
    /// Returns the resulting status (also stored on `self`). `None` threshold
    /// (decision not permitted in this phase) is treated as a failure.
    pub fn close(
        &mut self,
        tally: Tally,
        established_voters: u64,
        phase: Phase,
        closed_at: Timestamp,
    ) -> ProposalStatus {
        let class = self.kind.decision_class();
        let status = match threshold_for(class, phase) {
            None => ProposalStatus::Failed,
            Some(threshold) => {
                if decide(tally, established_voters, threshold).passed {
                    let effective_at = if class == DecisionClass::Constitutional {
                        closed_at.plus_days(RECALL_WINDOW_DAYS)
                    } else {
                        closed_at
                    };
                    ProposalStatus::Passed { effective_at }
                } else {
                    ProposalStatus::Failed
                }
            }
        };
        self.status = status;
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FranchiseCriteria;

    const DAY: i64 = Timestamp::SECONDS_PER_DAY;

    fn amend_proposal() -> Proposal {
        Proposal::new(
            ProposalId(1),
            DemosId(1),
            UserId(1),
            ProposalKind::AmendCriteria {
                proposed: FranchiseCriteria::platform_default(),
            },
            Timestamp(0),
            Timestamp(3 * DAY),
        )
    }

    #[test]
    fn passing_amendment_is_timelocked_past_the_recall_window() {
        let mut p = amend_proposal();
        let closed_at = Timestamp(3 * DAY);
        let status = p.close(Tally { aye: 80, nay: 20 }, 100, Phase::Sovereign, closed_at);
        match status {
            ProposalStatus::Passed { effective_at } => {
                // Effective only 7 days after close, not at close.
                assert_eq!(effective_at, closed_at.plus_days(RECALL_WINDOW_DAYS));
            }
            other => panic!("expected Passed, got {other:?}"),
        }
    }

    #[test]
    fn amendment_in_seed_phase_cannot_pass() {
        let mut p = amend_proposal();
        // Unanimous, full turnout — but Seed forbids constitutional change.
        let status = p.close(Tally { aye: 5, nay: 0 }, 5, Phase::Seed, Timestamp(3 * DAY));
        assert_eq!(status, ProposalStatus::Failed);
    }
}
