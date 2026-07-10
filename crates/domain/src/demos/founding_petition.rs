//! A demos that has been proposed but not yet founded.

use serde::{Deserialize, Serialize};

use crate::{FoundingId, Timestamp, UserId, SIGN_OFFS_REQUIRED};

/// A demos that has been proposed but not yet founded. It becomes a real
/// [`crate::Demos`] only once [`SIGN_OFFS_REQUIRED`] other people have signed off; until
/// then it holds just the founder's intent and the co-signers gathered so far.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FoundingPetition {
    pub id: FoundingId,
    /// The slug the demos will take, derived from `name` when the petition opened.
    pub slug: String,
    pub name: String,
    pub founder: UserId,
    /// Distinct co-signers, in the order they signed. Never includes the founder.
    pub sign_offs: Vec<UserId>,
    pub created_at: Timestamp,
}

impl FoundingPetition {
    /// Whether enough co-signers have committed for the demos to be founded.
    pub fn is_ready(&self) -> bool {
        self.sign_offs.len() >= SIGN_OFFS_REQUIRED
    }

    /// Co-signers still needed to reach quorum.
    pub fn remaining(&self) -> usize {
        SIGN_OFFS_REQUIRED.saturating_sub(self.sign_offs.len())
    }

    /// Whether `user` has already signed off (the founder never counts as a signer).
    pub fn is_signed_by(&self, user: UserId) -> bool {
        self.sign_offs.contains(&user)
    }
}
