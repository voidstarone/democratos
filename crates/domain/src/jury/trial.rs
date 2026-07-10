//! A jury trial.

use serde::{Deserialize, Serialize};

use crate::{DemosId, ReportId, Timestamp, TrialId, UserId, Verdict};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Trial {
    pub id: TrialId,
    pub demos_id: DemosId,
    pub report_id: ReportId,
    pub accused: UserId,
    pub jurors: Vec<UserId>,
    /// The total vote weight of the empanelled jury, frozen at selection. The
    /// conviction bar (2/3 supermajority) is measured against this, so it stays
    /// fixed even if a juror's weight later changes. Under one-juror-one-vote it
    /// equals `jurors.len()`. `#[serde(default)]` reads older trials as `0`
    /// (which auto-acquits — harmless for the short-lived open trials affected).
    #[serde(default)]
    pub jury_weight: u64,
    /// Each juror's vote weight, frozen at empanelment and aligned by index with
    /// `jurors`. The verdict tally weighs each ballot by the juror's *frozen*
    /// weight here — not a live recomputation — so the guilty/nay sums and the
    /// `jury_weight` denominator share a single basis and a juror cannot shift the
    /// 2/3 conviction bar mid-trial by pumping their contribution. Empty for older
    /// trials and for one-juror-one-vote juries, where [`Trial::juror_weight`]
    /// returns 1.
    #[serde(default)]
    pub juror_weights: Vec<u64>,
    pub opened_at: Timestamp,
    pub closes_at: Timestamp,
    pub verdict: Verdict,
    /// Optimistic-concurrency revision — see [`crate::Proposal::rev`]. Guards the
    /// read-modify-write that records a verdict against a concurrent settle on
    /// another replica silently clobbering it.
    #[serde(default)]
    pub rev: u64,
}

impl Trial {
    pub fn new(
        id: TrialId,
        demos_id: DemosId,
        report_id: ReportId,
        accused: UserId,
        jurors: Vec<UserId>,
        jury_weight: u64,
        juror_weights: Vec<u64>,
        opened_at: Timestamp,
        closes_at: Timestamp,
    ) -> Self {
        Self {
            id,
            demos_id,
            report_id,
            accused,
            jurors,
            jury_weight,
            juror_weights,
            opened_at,
            closes_at,
            verdict: Verdict::Pending,
            rev: 0,
        }
    }

    pub fn is_juror(&self, user: UserId) -> bool {
        self.jurors.contains(&user)
    }

    /// This juror's vote weight, frozen at empanelment. Returns 1 when the trial
    /// carries no frozen weights (older data or an unweighted jury) or the user
    /// was not on the panel.
    pub fn juror_weight(&self, user: UserId) -> u64 {
        self.jurors
            .iter()
            .position(|j| *j == user)
            .and_then(|i| self.juror_weights.get(i).copied())
            .unwrap_or(1)
    }
}
