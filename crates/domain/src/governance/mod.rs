//! Layers 3 & 4 of the defense: tiered decision thresholds (how hard a decision
//! is to pass) and the timelock (a passed constitutional change does not take
//! effect immediately, leaving a recall window).

pub mod decide;
pub mod decision;
pub mod decision_class;
pub mod proposal;
pub mod proposal_kind;
pub mod proposal_status;
pub mod recall_window_days;
pub mod tally;
pub mod threshold;
pub mod threshold_for;
