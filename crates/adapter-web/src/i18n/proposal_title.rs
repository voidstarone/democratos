use crate::i18n::lang::Lang;
use crate::i18n::posting_policy_label::posting_policy_label;
use domain::{JurySizing, ProposalKind, VoteWeighting, WeightingScope};

pub fn proposal_title(lang: Lang, kind: &ProposalKind) -> String {
    match (lang, kind) {
        (Lang::En, ProposalKind::RemoveContent { target }) => format!("Remove {target}"),
        (Lang::En, ProposalKind::Ban { user }) => format!("Ban user #{user}"),
        (Lang::En, ProposalKind::Recall { leader }) => format!("Recall leader #{leader}"),
        (Lang::En, ProposalKind::AmendCriteria { proposed }) => format!(
            "Amend criteria → age {}, member {}, contrib {}",
            proposed.min_account_age_days, proposed.min_membership_days, proposed.min_contribution
        ),
        (Lang::Es, ProposalKind::RemoveContent { target }) => format!("Eliminar {target}"),
        (Lang::Es, ProposalKind::Ban { user }) => format!("Expulsar al usuario #{user}"),
        (Lang::Es, ProposalKind::Recall { leader }) => format!("Revocar al líder #{leader}"),
        (Lang::Es, ProposalKind::AmendCriteria { proposed }) => format!(
            "Enmendar criterios → antigüedad {}, miembro {}, contrib {}",
            proposed.min_account_age_days, proposed.min_membership_days, proposed.min_contribution
        ),
        (Lang::En, ProposalKind::AddRule { text, sanction_days }) => {
            format!("Add rule ({}): {text}", ban_term_label_en(*sanction_days))
        }
        (Lang::En, ProposalKind::RemoveRule { rule }) => format!("Repeal rule #{rule}"),
        (Lang::Es, ProposalKind::AddRule { text, sanction_days }) => {
            format!("Añadir regla ({}): {text}", ban_term_label_es(*sanction_days))
        }
        (Lang::Es, ProposalKind::RemoveRule { rule }) => format!("Derogar la regla #{rule}"),
        (Lang::En, ProposalKind::SetMaxSanction { days }) => {
            format!("Set ban ceiling → {days} days")
        }
        (Lang::Es, ProposalKind::SetMaxSanction { days }) => {
            format!("Límite de expulsión → {days} días")
        }
        (Lang::En, ProposalKind::SetNsfwPolicy { allows_nsfw }) => {
            format!(
                "Set NSFW policy → {}",
                if *allows_nsfw { "allow" } else { "forbid" }
            )
        }
        (Lang::Es, ProposalKind::SetNsfwPolicy { allows_nsfw }) => format!(
            "Política NSFW → {}",
            if *allows_nsfw { "permitir" } else { "prohibir" }
        ),
        (Lang::En, ProposalKind::SetJurySizing { sizing }) => {
            format!("Set jury sizing → {}", jury_sizing_label(*sizing))
        }
        (Lang::Es, ProposalKind::SetJurySizing { sizing }) => {
            format!("Tamaño del jurado → {}", jury_sizing_label(*sizing))
        }
        (Lang::En, ProposalKind::SetVoteWeighting { scheme }) => {
            format!("Set vote weighting → {}", vote_weighting_label(*scheme))
        }
        (Lang::Es, ProposalKind::SetVoteWeighting { scheme }) => {
            format!("Ponderación del voto → {}", vote_weighting_label(*scheme))
        }
        (Lang::En, ProposalKind::SetWeightingScope { scope }) => {
            format!("Set weighting scope → {}", weighting_scope_label(*scope))
        }
        (Lang::Es, ProposalKind::SetWeightingScope { scope }) => {
            format!("Alcance de ponderación → {}", weighting_scope_label(*scope))
        }
        (Lang::En, ProposalKind::GrantVoteWeight { user, weight }) => {
            format!("Grant user #{user} vote weight {weight}")
        }
        (Lang::Es, ProposalKind::GrantVoteWeight { user, weight }) => {
            format!("Otorgar al usuario #{user} peso de voto {weight}")
        }
        (Lang::En, ProposalKind::SetPostingPolicy { policy }) => {
            format!(
                "Set posting policy → {}",
                posting_policy_label(lang, *policy)
            )
        }
        (Lang::Es, ProposalKind::SetPostingPolicy { policy }) => {
            format!(
                "Política de publicación → {}",
                posting_policy_label(lang, *policy)
            )
        }
    }
}

/// Compact, language-neutral renderings of the governance policies, reused by
/// both locales in [`proposal_title`].
fn jury_sizing_label(sizing: JurySizing) -> String {
    match sizing {
        JurySizing::Sqrt {
            post_factor_bp,
            comment_factor_bp,
        } => {
            format!("√n (post {post_factor_bp}bp, comment {comment_factor_bp}bp)")
        }
        JurySizing::Proportion {
            post_bp,
            comment_bp,
        } => {
            format!(
                "post {}% / comment {}% of voters",
                post_bp / 100,
                comment_bp / 100
            )
        }
        JurySizing::Fixed { post, comment } => format!("{post} post / {comment} comment"),
    }
}

/// How a rule's ban term reads in a proposal title. `0` means the rule inherits
/// the community ceiling rather than naming its own term.
fn ban_term_label_en(days: u32) -> String {
    if days == 0 {
        "ban: community max".to_string()
    } else {
        format!("ban: {days} days")
    }
}

fn ban_term_label_es(days: u32) -> String {
    if days == 0 {
        "expulsión: máx. comunidad".to_string()
    } else {
        format!("expulsión: {days} días")
    }
}

fn vote_weighting_label(scheme: VoteWeighting) -> &'static str {
    match scheme {
        VoteWeighting::Equal => "equal",
        VoteWeighting::ByContribution => "by contribution",
        VoteWeighting::ByTenure => "by tenure",
        VoteWeighting::ByRole => "by role",
    }
}

fn weighting_scope_label(scope: WeightingScope) -> &'static str {
    match scope {
        WeightingScope::Both => "juries + proposals",
        WeightingScope::JuriesOnly => "juries only",
        WeightingScope::ProposalsOnly => "proposals only",
        WeightingScope::None => "off",
    }
}
