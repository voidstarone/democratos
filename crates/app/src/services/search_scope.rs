//! Where a search looks.

use domain::DemosId;

/// Where a search looks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SearchScope {
    /// Across every community.
    All,
    /// Restricted to a single community.
    Demos(DemosId),
}
