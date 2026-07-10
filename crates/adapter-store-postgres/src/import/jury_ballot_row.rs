//! A jury ballot to import.

/// A jury ballot to import.
pub struct JuryBallotRow {
    pub trial: u64,
    pub juror: u64,
    pub guilty: bool,
    pub weight: u64,
}
