//! Automatic bot detection.
//!
//! A pure, auditable heuristic over behavioural signals. It produces a 0–100
//! suspicion score; above [`crate::BOT_REPORT_THRESHOLD`] the application files an
//! *automatic report* (it never auto-punishes — a jury decides). Keeping this a
//! pure function means the score is reproducible and testable, and the exact
//! weighting can be reviewed (and later tuned by the community).

pub mod bot_report_threshold;
pub mod bot_score;
pub mod bot_signals;
pub mod is_likely_bot;
