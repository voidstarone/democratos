//! One row in a community's public case log.

/// One trial in a community's case log: who was accused and how it ended (or that
/// it's still open). Public record — links to the full, public trial page.
pub struct CaseRow {
    pub trial_id: u64,
    pub accused: String,
    /// Localized verdict label (Pending / Guilty / Not guilty).
    pub verdict: String,
    /// Whether the trial is still open (verdict pending), for styling.
    pub open: bool,
}
