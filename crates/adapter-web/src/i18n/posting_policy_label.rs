use crate::i18n::lang::Lang;
use domain::PostingPolicy;

/// A human label for a posting policy, in the given language.
pub fn posting_policy_label(lang: Lang, policy: PostingPolicy) -> String {
    match (lang, policy) {
        (Lang::En, PostingPolicy::Open) => "anyone".into(),
        (Lang::En, PostingPolicy::Members) => "members".into(),
        (Lang::En, PostingPolicy::Voters) => "voters only".into(),
        (Lang::En, PostingPolicy::MinContribution(n)) => format!("popularity ≥ {n}"),
        (Lang::Es, PostingPolicy::Open) => "cualquiera".into(),
        (Lang::Es, PostingPolicy::Members) => "miembros".into(),
        (Lang::Es, PostingPolicy::Voters) => "solo votantes".into(),
        (Lang::Es, PostingPolicy::MinContribution(n)) => format!("popularidad ≥ {n}"),
    }
}
