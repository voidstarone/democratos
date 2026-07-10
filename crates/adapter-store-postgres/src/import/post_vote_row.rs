//! A post up/down vote to import.

/// A post up/down vote to import.
pub struct PostVoteRow {
    pub post: u64,
    pub user: u64,
    pub up: bool,
}
