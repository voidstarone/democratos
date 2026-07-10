//! The canonical message a user signs to cast a governance ballot on a proposal.

use crate::identity::domain::DOMAIN;

/// The canonical message a user signs to cast a governance ballot on a proposal.
pub fn vote_message(proposal: u64, aye: bool) -> String {
    format!(
        "{DOMAIN}:vote:{proposal}:{}",
        if aye { "aye" } else { "nay" }
    )
}
