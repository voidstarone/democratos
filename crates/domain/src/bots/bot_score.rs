//! The bot suspicion score.

use crate::BotSignals;

/// Suspicion score, 0 (clearly human) to 100 (almost certainly a bot).
pub fn bot_score(s: &BotSignals) -> u8 {
    let mut score: u32 = 0;

    // Very young accounts are cheap to mint and over-represented among bots.
    if s.account_age_days < 2 {
        score += 30;
    } else if s.account_age_days < 7 {
        score += 15;
    }

    // Inhuman posting cadence.
    if s.actions_last_hour > 30 {
        score += 30;
    } else if s.actions_last_hour > 10 {
        score += 15;
    }

    // Copy-paste repetition.
    if s.duplicate_actions >= 3 {
        score += 25;
    } else if s.duplicate_actions >= 1 {
        score += 10;
    }

    // Cross-posting the same payload widely.
    if s.demos_spammed >= 5 {
        score += 20;
    } else if s.demos_spammed >= 3 {
        score += 10;
    }

    score.min(100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{is_likely_bot, BOT_REPORT_THRESHOLD};

    #[test]
    fn established_active_human_scores_low() {
        let human = BotSignals {
            account_age_days: 400,
            actions_last_hour: 4,
            duplicate_actions: 0,
            demos_spammed: 1,
        };
        assert!(bot_score(&human) < BOT_REPORT_THRESHOLD);
        assert!(!is_likely_bot(&human));
    }

    #[test]
    fn fresh_spraying_spammer_is_flagged() {
        let bot = BotSignals {
            account_age_days: 0,
            actions_last_hour: 50,
            duplicate_actions: 8,
            demos_spammed: 6,
        };
        // 30 + 30 + 25 + 20 = 105 -> clamped to 100.
        assert_eq!(bot_score(&bot), 100);
        assert!(is_likely_bot(&bot));
    }

    #[test]
    fn borderline_is_not_over_flagged() {
        // Young-ish + slightly fast, nothing else: should stay under threshold.
        let s = BotSignals {
            account_age_days: 5,
            actions_last_hour: 12,
            duplicate_actions: 0,
            demos_spammed: 1,
        };
        assert_eq!(bot_score(&s), 30); // 15 + 15
        assert!(!is_likely_bot(&s));
    }
}
