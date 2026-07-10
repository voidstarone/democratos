use crate::i18n::lang::Lang;

pub fn verdict(lang: Lang, v: domain::Verdict) -> &'static str {
    use domain::Verdict::*;
    match (lang, v) {
        (Lang::En, Pending) => "pending",
        (Lang::En, Guilty) => "guilty",
        (Lang::En, NotGuilty) => "not guilty",
        (Lang::Es, Pending) => "pendiente",
        (Lang::Es, Guilty) => "culpable",
        (Lang::Es, NotGuilty) => "no culpable",
    }
}
