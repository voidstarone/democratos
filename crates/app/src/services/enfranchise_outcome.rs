//! Result of a member asking to be admitted to the franchise.

use domain::Eligibility;

/// Result of a member asking to be admitted to the franchise.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EnfranchiseOutcome {
    /// Admitted as a voter.
    Admitted,
    /// Eligible, but the demos has no admission slots this 30-day window
    /// (Layer 2). The member keeps their place in the qualification queue.
    Queued,
    /// Not yet eligible (Layer 1); carries the unmet requirements.
    NotEligible(Eligibility),
}
