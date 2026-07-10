use crate::i18n::lang::Lang;
use domain::ProposalStatus;

pub fn status(lang: Lang, status: &ProposalStatus) -> &'static str {
    match (lang, status) {
        (Lang::En, ProposalStatus::Open) => "open",
        (Lang::En, ProposalStatus::Passed { .. }) => "passed",
        (Lang::En, ProposalStatus::Failed) => "failed",
        (Lang::Es, ProposalStatus::Open) => "abierta",
        (Lang::Es, ProposalStatus::Passed { .. }) => "aprobada",
        (Lang::Es, ProposalStatus::Failed) => "rechazada",
    }
}
