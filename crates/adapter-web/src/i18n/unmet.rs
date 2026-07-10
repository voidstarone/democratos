use crate::i18n::lang::Lang;
use domain::Unmet;

pub fn unmet(lang: Lang, unmet: &Unmet) -> String {
    match (lang, unmet) {
        (
            Lang::En,
            Unmet::AccountTooYoung {
                need_days,
                have_days,
            },
        ) => {
            format!("account age: need {need_days} days, have {have_days}")
        }
        (
            Lang::En,
            Unmet::MembershipTooShort {
                need_days,
                have_days,
            },
        ) => {
            format!("membership: need {need_days} days, have {have_days}")
        }
        (Lang::En, Unmet::InsufficientContribution { need, have }) => {
            format!("contribution: need {need}, have {have}")
        }
        (Lang::En, Unmet::Sanctioned) => "under an active sanction".to_string(),
        (Lang::En, Unmet::Barred) => "this account cannot hold the franchise".to_string(),
        (
            Lang::Es,
            Unmet::AccountTooYoung {
                need_days,
                have_days,
            },
        ) => {
            format!("antigüedad de cuenta: necesitas {need_days} días, tienes {have_days}")
        }
        (
            Lang::Es,
            Unmet::MembershipTooShort {
                need_days,
                have_days,
            },
        ) => {
            format!("membresía: necesitas {need_days} días, tienes {have_days}")
        }
        (Lang::Es, Unmet::InsufficientContribution { need, have }) => {
            format!("contribución: necesitas {need}, tienes {have}")
        }
        (Lang::Es, Unmet::Sanctioned) => "bajo una sanción activa".to_string(),
        (Lang::Es, Unmet::Barred) => "esta cuenta no puede tener el sufragio".to_string(),
    }
}
