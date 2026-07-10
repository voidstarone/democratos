//! Behavioural signals for bot detection.

use serde::{Deserialize, Serialize};

/// Behavioural signals the application gathers about an account's recent
/// activity in a demos. The domain only consumes them; how they are measured is
/// the adapter's concern.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct BotSignals {
    pub account_age_days: i64,
    /// Posts + comments in the last hour.
    pub actions_last_hour: u32,
    /// Identical bodies repeated (copy-paste spam).
    pub duplicate_actions: u32,
    /// Distinct demos the same content was sprayed across.
    pub demos_spammed: u32,
}
