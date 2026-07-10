//! How many rows of each kind an import actually inserted.

/// How many rows of each kind were actually inserted (a re-import reports `0`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImportCounts {
    pub users: u64,
    pub demoi: u64,
    pub memberships: u64,
    pub proposals: u64,
    pub votes: u64,
    pub post_votes: u64,
    pub rules: u64,
    pub posts: u64,
    pub comments: u64,
    pub reports: u64,
    pub trials: u64,
    pub jury_ballots: u64,
}

impl ImportCounts {
    /// Total rows inserted across all kinds.
    pub fn total(&self) -> u64 {
        self.users
            + self.demoi
            + self.memberships
            + self.proposals
            + self.votes
            + self.post_votes
            + self.rules
            + self.posts
            + self.comments
            + self.reports
            + self.trials
            + self.jury_ballots
    }
}
