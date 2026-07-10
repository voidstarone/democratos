//! Which collective decisions a demos applies its vote weighting to.

use serde::{Deserialize, Serialize};

/// Which collective decisions a demos applies its [`crate::VoteWeighting`] to. Changed
/// via [`crate::ProposalKind::SetWeightingScope`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum WeightingScope {
    /// Both jury verdicts and governance proposals are weighted.
    #[default]
    Both,
    /// Only jury verdicts are weighted; proposals stay one-citizen-one-vote.
    JuriesOnly,
    /// Only governance proposals are weighted; juries stay one-juror-one-vote.
    ProposalsOnly,
    /// Weighting is ignored — one-citizen-one-vote everywhere.
    None,
}

impl WeightingScope {
    pub fn applies_to_juries(&self) -> bool {
        matches!(self, WeightingScope::Both | WeightingScope::JuriesOnly)
    }

    pub fn applies_to_proposals(&self) -> bool {
        matches!(self, WeightingScope::Both | WeightingScope::ProposalsOnly)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_predicates() {
        assert!(WeightingScope::Both.applies_to_juries());
        assert!(WeightingScope::Both.applies_to_proposals());
        assert!(WeightingScope::JuriesOnly.applies_to_juries());
        assert!(!WeightingScope::JuriesOnly.applies_to_proposals());
        assert!(!WeightingScope::None.applies_to_juries());
        assert!(!WeightingScope::None.applies_to_proposals());
    }
}
