use serde::{Deserialize, Serialize};

use domain::Verdict;

/// The result of executing a [`Command`](crate::Command) on the owner.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CommandOutcome {
    Voted,
    /// The post's new net score.
    PostScore(i64),
    /// The trial's verdict after the ballot.
    Verdict(Verdict),
    /// A trusted issuer minted an account; carries the new account's global id so the
    /// requesting node can start a session for it.
    AccountMinted { id: u64 },
    /// A home issuer verified a delegated login; carries the account id so the
    /// requesting node can start a session for it.
    Authenticated { id: u64 },
}
