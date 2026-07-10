//! The community page handler (plus its feed-fragment and view builders).

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Response,
};
use domain::{enfranchisement_slots, evaluate_eligibility, Phase, Timestamp, User};

use crate::handlers::can_vote::can_vote;
use crate::handlers::current_user::current_user;
use crate::handlers::demos_posts_page::demos_posts_page;
use crate::handlers::feed_query::FeedQuery;
use crate::handlers::make_pager::make_pager;
use crate::handlers::page_of::page_of;
use crate::handlers::paging_mode::paging_mode;
use crate::handlers::post_row::post_row;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::wants_fragment::wants_fragment;
use crate::i18n::lang::Lang;
use crate::views::demos_feed_fragment_view::DemosFeedFragmentView;
use crate::views::demos_view::DemosView;
use crate::views::rule_view::RuleView;
use crate::views::standing_view::StandingView;
use crate::AppState;

pub async fn demos_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Query(pg): Query<FeedQuery>,
) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    let page = page_of(pg.page);
    let mode = paging_mode(user.as_ref());

    // Lazy-load: return only the next slice of this community's posts.
    if wants_fragment(&headers) {
        return match demos_feed_fragment(&state, lang, &slug, user.as_ref(), page, mode).await {
            Ok(resp) => resp,
            Err(e) => render_error(lang, user.map(|u| u.handle), e.to_string()),
        };
    }

    match build_demos_view(&state, lang, &slug, user.as_ref(), page, mode).await {
        Ok(view) => render(view),
        Err(e) => render_error(lang, user.map(|u| u.handle), e.to_string()),
    }
}

/// The bare paginated post slice for a community, for the lazy-loader. Reuses the
/// same ordering and votability rule as the full page so appended cards match.
async fn demos_feed_fragment(
    state: &AppState,
    lang: Lang,
    slug: &str,
    viewer: Option<&User>,
    page: usize,
    mode: &'static str,
) -> app::Result<Response> {
    let demos = state
        .services
        .demoi
        .by_slug(slug)
        .await?
        .ok_or(app::StoreError::NotFound)?;
    let viewer_id = viewer.map(|u| u.id);
    let votable = can_vote(state, viewer_id, demos.id).await;
    // A voter's appended cards keep the "propose removal" action, matching page 1.
    let viewer_is_voter = match (viewer_id, votable) {
        (Some(uid), _) => state
            .services
            .memberships
            .get(uid, demos.id)
            .await
            .ok()
            .flatten()
            .map(|m| m.is_voter())
            .unwrap_or(false),
        _ => false,
    };
    let (window, has_next) = demos_posts_page(state, demos.id, page).await?;
    let mut posts = Vec::new();
    for p in window {
        posts.push(post_row(state, &p, viewer_id, votable, None).await);
    }
    let pager = make_pager(&format!("/d/{}?", demos.slug), page, has_next, mode);
    Ok(render(DemosFeedFragmentView {
        t: lang.strings(),
        posts,
        pager,
        slug: demos.slug.clone(),
        viewer_is_voter,
    }))
}

async fn build_demos_view(
    state: &AppState,
    lang: Lang,
    slug: &str,
    viewer: Option<&User>,
    page: usize,
    mode: &'static str,
) -> app::Result<DemosView> {
    let demos = state
        .services
        .demoi
        .by_slug(slug)
        .await?
        .ok_or(app::StoreError::NotFound)?;
    let now = state.services.clock.now();
    let voters = state.services.memberships.voter_count(demos.id).await?;
    let phase = Phase::from_voter_count(voters);

    let mut viewer_is_voter = false;
    let mut viewer_can_post = false;
    let standing = if let Some(u) = viewer {
        match state.services.memberships.get(u.id, demos.id).await? {
            None => Some(StandingView {
                joined: false,
                tier: String::new(),
                is_voter: false,
                eligible: false,
                contribution: 0,
                unmet: vec![],
                queued_note: None,
            }),
            Some(m) if m.is_voter() => {
                viewer_is_voter = true;
                viewer_can_post = !m.sanctioned;
                Some(StandingView {
                    joined: true,
                    tier: crate::i18n::tier::tier(lang, m.tier).to_string(),
                    is_voter: true,
                    eligible: true,
                    contribution: m.contribution,
                    unmet: vec![],
                    queued_note: None,
                })
            }
            Some(m) => {
                viewer_can_post = !m.sanctioned;
                let elig = evaluate_eligibility(u, &m, &demos.criteria, now);
                let eligible = elig.is_eligible();
                let unmet = elig
                    .unmet
                    .iter()
                    .map(|x| crate::i18n::unmet::unmet(lang, x))
                    .collect();
                let queued_note = if eligible {
                    let window_start = Timestamp(now.0 - 30 * Timestamp::SECONDS_PER_DAY);
                    let admitted = state
                        .services
                        .memberships
                        .admitted_since(demos.id, window_start)
                        .await?;
                    (enfranchisement_slots(voters, admitted) == 0)
                        .then(|| crate::i18n::queued_note::queued_note(lang))
                } else {
                    None
                };
                Some(StandingView {
                    joined: true,
                    tier: crate::i18n::tier::tier(lang, m.tier).to_string(),
                    is_voter: false,
                    eligible,
                    contribution: m.contribution,
                    unmet,
                    queued_note,
                })
            }
        }
    } else {
        None
    };

    let rules = state
        .services
        .list_rules(demos.id)
        .await?
        .into_iter()
        .map(|r| RuleView {
            id: r.id.0,
            text: r.text,
        })
        .collect();

    // A member in good standing of this community may up/down vote its posts.
    let votable = viewer_can_post;
    let viewer_id = viewer.map(|u| u.id);
    let (window, has_next) = demos_posts_page(state, demos.id, page).await?;
    let mut posts = Vec::new();
    for p in window {
        posts.push(post_row(state, &p, viewer_id, votable, None).await);
    }
    let pager = make_pager(&format!("/d/{}?", demos.slug), page, has_next, mode);

    Ok(DemosView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: viewer.map(|u| u.handle.clone()),
        slug: demos.slug.clone(),
        name: demos.name.clone(),
        phase: crate::i18n::phase::phase(lang, phase).to_string(),
        voters,
        criteria_age: demos.criteria.min_account_age_days,
        criteria_member: demos.criteria.min_membership_days,
        criteria_contrib: demos.criteria.min_contribution,
        can_amend: phase != Phase::Seed,
        viewer_is_voter,
        viewer_can_post,
        standing,
        rules,
        posts,
        pager,
    })
}
