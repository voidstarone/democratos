//! A platform-wide review case gathering reviewer classifications.

use serde::{Deserialize, Serialize};

use crate::sensitive::review_vote::ReviewVote;
use crate::sensitive::sensitive_case_status::SensitiveCaseStatus;
use crate::sensitive::sensitive_tag::SensitiveTag;
use crate::sensitive::tally_tags::tally_tags;
use crate::{ReportTarget, SensitiveCaseId, Timestamp, UserId};

/// A **platform-wide** (deliberately extra-demos) case gathering reviewer
/// classifications of one flagged item. It is opened by the first flag and gathers
/// [`ReviewVote`]s while [`Open`](SensitiveCaseStatus::Open); once at least
/// [`REVIEW_QUORUM`](crate::REVIEW_QUORUM) reviewers have classified it, the
/// plurality tag resolves it. Unlike a [`Report`](crate::Report) this is not tied
/// to a demos and is not judged by a community jury — sensitive/illegal content is
/// not a matter of community opinion.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SensitiveCase {
    pub id: SensitiveCaseId,
    pub target: ReportTarget,
    /// Who first flagged it (`None` if system-filed).
    pub reporter: Option<UserId>,
    pub note: String,
    pub created_at: Timestamp,
    pub votes: Vec<ReviewVote>,
    pub status: SensitiveCaseStatus,
    /// Optimistic-concurrency revision — guards the read-modify-write when a vote
    /// is added on one replica against a concurrent update on another.
    #[serde(default)]
    pub rev: u64,
}

impl SensitiveCase {
    /// Open a fresh case for `target`, flagged by `reporter`.
    pub fn new(
        id: SensitiveCaseId,
        target: ReportTarget,
        reporter: Option<UserId>,
        note: impl Into<String>,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            target,
            reporter,
            note: note.into(),
            created_at,
            votes: Vec::new(),
            status: SensitiveCaseStatus::Open,
            rev: 0,
        }
    }

    /// Record `reviewer`'s classification. One vote per reviewer: a repeat replaces
    /// the earlier tag (a reviewer may correct themselves before the case resolves).
    pub fn cast(&mut self, reviewer: UserId, tag: SensitiveTag, at: Timestamp) {
        if let Some(existing) = self.votes.iter_mut().find(|v| v.reviewer == reviewer) {
            existing.tag = tag;
            existing.at = at;
        } else {
            self.votes.push(ReviewVote {
                reviewer,
                tag,
                at,
            });
        }
    }

    /// How many distinct reviewers have classified this case.
    pub fn reviewer_count(&self) -> usize {
        self.votes.len()
    }

    /// Whether `reviewer` has already voted.
    pub fn has_voted(&self, reviewer: UserId) -> bool {
        self.votes.iter().any(|v| v.reviewer == reviewer)
    }

    /// The winning tag if the quorum is met, else `None`.
    pub fn winning_tag(&self) -> Option<SensitiveTag> {
        let tags: Vec<SensitiveTag> = self.votes.iter().map(|v| v.tag).collect();
        tally_tags(&tags)
    }

    /// Resolve the case if the quorum is met, stamping the winning tag on its
    /// status. Returns the winning tag when this call resolves it (idempotent: a
    /// case already resolved returns `None`).
    pub fn try_resolve(&mut self) -> Option<SensitiveTag> {
        if self.status != SensitiveCaseStatus::Open {
            return None;
        }
        let winner = self.winning_tag()?;
        self.status = SensitiveCaseStatus::Resolved(winner);
        Some(winner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PostId;
    use SensitiveTag::*;

    fn case() -> SensitiveCase {
        SensitiveCase::new(
            SensitiveCaseId(1),
            ReportTarget::Post(PostId(7)),
            Some(UserId(1)),
            "looks bad",
            Timestamp(0),
        )
    }

    #[test]
    fn a_reviewer_votes_once_and_can_correct() {
        let mut c = case();
        c.cast(UserId(2), Porn, Timestamp(1));
        c.cast(UserId(2), Gore, Timestamp(2)); // same reviewer corrects
        assert_eq!(c.reviewer_count(), 1);
        assert_eq!(c.votes[0].tag, Gore);
    }

    #[test]
    fn resolves_only_at_quorum_and_is_idempotent() {
        let mut c = case();
        for (i, tag) in [Gore, Gore, Gore, Porn].into_iter().enumerate() {
            c.cast(UserId(i as u64 + 10), tag, Timestamp(1));
        }
        assert_eq!(c.try_resolve(), None); // only 4 reviewers
        c.cast(UserId(99), Porn, Timestamp(1)); // 5th
        assert_eq!(c.try_resolve(), Some(Gore));
        assert_eq!(c.status, SensitiveCaseStatus::Resolved(Gore));
        assert_eq!(c.try_resolve(), None); // already resolved
    }
}
