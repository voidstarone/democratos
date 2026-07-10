//! The automatic-report predicate for bot signals.

use crate::{bot_score, BotSignals, BOT_REPORT_THRESHOLD};

/// Whether the signals warrant an automatic report.
pub fn is_likely_bot(s: &BotSignals) -> bool {
    bot_score(s) >= BOT_REPORT_THRESHOLD
}
