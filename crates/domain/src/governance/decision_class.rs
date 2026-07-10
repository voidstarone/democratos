//! The weight-classes of a governance decision.

use serde::{Deserialize, Serialize};

/// The weight-classes of decision, in rough ascending order of how hard they
/// are to pass.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DecisionClass {
    Moderation,
    /// Add or repeal a community rule. Allowed in every phase (including Seed),
    /// so a founding community can write its rulebook.
    RuleChange,
    BanOrRecall,
    Constitutional,
}
