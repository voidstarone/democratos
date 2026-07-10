use crate::i18n::lang::Lang;
use domain::ProposalKind;

pub fn class(lang: Lang, kind: &ProposalKind) -> &'static str {
    use domain::DecisionClass::*;
    match (lang, kind.decision_class()) {
        (Lang::En, Moderation) => "moderation",
        (Lang::En, RuleChange) => "rule change",
        (Lang::En, BanOrRecall) => "ban / recall",
        (Lang::En, Constitutional) => "constitutional",
        (Lang::Es, Moderation) => "moderación",
        (Lang::Es, RuleChange) => "cambio de regla",
        (Lang::Es, BanOrRecall) => "expulsión / revocación",
        (Lang::Es, Constitutional) => "constitucional",
    }
}
