//! Localized rendering of a rule's ban term for the rules list.

use crate::i18n::lang::Lang;

/// How a rule's ban term reads in the rules list. `0` days means the rule
/// inherits the community ceiling rather than naming its own term.
pub fn rule_ban_term(lang: Lang, sanction_days: u32) -> String {
    let s = lang.strings();
    if sanction_days == 0 {
        s.rule_ban_inherits.to_string()
    } else {
        format!("{} {} d", s.rule_ban_prefix, sanction_days)
    }
}
