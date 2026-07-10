use crate::i18n::lang::Lang;
use domain::Phase;

pub fn phase(lang: Lang, phase: Phase) -> &'static str {
    match (lang, phase) {
        (Lang::En, Phase::Seed) => "Seed",
        (Lang::En, Phase::Chartering) => "Chartering",
        (Lang::En, Phase::Sovereign) => "Sovereign",
        (Lang::Es, Phase::Seed) => "Semilla",
        (Lang::Es, Phase::Chartering) => "Constituyente",
        (Lang::Es, Phase::Sovereign) => "Soberano",
    }
}
