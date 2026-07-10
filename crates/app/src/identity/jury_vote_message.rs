//! The canonical message a user signs to cast a juror's verdict ballot.

use crate::identity::domain::DOMAIN;

/// The canonical message a user signs to cast a juror's verdict ballot.
pub fn jury_vote_message(trial: u64, guilty: bool) -> String {
    format!(
        "{DOMAIN}:jury:{trial}:{}",
        if guilty { "guilty" } else { "not_guilty" }
    )
}
