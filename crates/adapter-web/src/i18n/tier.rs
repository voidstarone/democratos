use crate::i18n::lang::Lang;
use domain::Tier;

pub fn tier(lang: Lang, tier: Tier) -> &'static str {
    match (lang, tier) {
        (Lang::En, Tier::Lurker) => "Lurker",
        (Lang::En, Tier::Member) => "Member",
        (Lang::En, Tier::Voter) => "Voter",
        (Lang::Es, Tier::Lurker) => "Observador",
        (Lang::Es, Tier::Member) => "Miembro",
        (Lang::Es, Tier::Voter) => "Votante",
    }
}
