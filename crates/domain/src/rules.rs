//! Community rules — the conduct/content policy a demos sets for itself.
//!
//! Rules are amended through the governance system ([`crate::ProposalKind::AddRule`]
//! / [`crate::ProposalKind::RemoveRule`], decision class
//! [`crate::DecisionClass::RuleChange`]). Unlike the franchise criteria, rules
//! may be set even in the Seed phase, so a founding community can establish its
//! rulebook from day one.

use serde::{Deserialize, Serialize};

use crate::{DemosId, RuleId};
use crate::time::Timestamp;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub id: RuleId,
    pub demos_id: DemosId,
    pub text: String,
    pub enacted_at: Timestamp,
    /// Repealed rules are kept for the record but no longer in force.
    pub active: bool,
}

impl Rule {
    pub fn new(
        id: RuleId,
        demos_id: DemosId,
        text: impl Into<String>,
        enacted_at: Timestamp,
    ) -> Self {
        Self {
            id,
            demos_id,
            text: text.into(),
            enacted_at,
            active: true,
        }
    }
}
