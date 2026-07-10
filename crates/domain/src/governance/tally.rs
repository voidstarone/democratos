//! The aye/nay counts for a proposal.

use serde::{Deserialize, Serialize};

/// The aye/nay counts for a proposal.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Tally {
    pub aye: u64,
    pub nay: u64,
}

impl Tally {
    pub fn cast(&self) -> u64 {
        self.aye + self.nay
    }
}
