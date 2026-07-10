//! A proposal vote to import.

/// A proposal vote to import, exactly as the text-file store persisted it.
pub struct VoteRow {
    pub proposal: u64,
    pub voter: u64,
    pub aye: bool,
    pub weight: u64,
}
