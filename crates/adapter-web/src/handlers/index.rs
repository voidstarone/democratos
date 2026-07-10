//! The home feed handler.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
};
use domain::Phase;

use crate::handlers::can_vote::can_vote;
use crate::handlers::current_user::current_user;
use crate::handlers::feed_page_size::FEED_PAGE_SIZE;
use crate::handlers::feed_query::FeedQuery;
use crate::handlers::make_pager::make_pager;
use crate::handlers::page_of::page_of;
use crate::handlers::paginate::paginate;
use crate::handlers::paging_mode::paging_mode;
use crate::handlers::post_row::post_row;
use crate::handlers::render::render;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::wants_fragment::wants_fragment;
use crate::i18n::phase::phase;
use crate::views::demos_list_item::DemosListItem;
use crate::views::feed_fragment_view::FeedFragmentView;
use crate::views::founding_list_item::FoundingListItem;
use crate::views::index_view::IndexView;
use crate::AppState;

pub async fn index(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(pg): Query<FeedQuery>,
) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    let viewer_id = user.as_ref().map(|u| u.id);
    let page = page_of(pg.page);
    let mode = paging_mode(user.as_ref());

    // The home syndicates popular content across every community. A signed-in
    // viewer gets the recommendation algorithm (item-based collaborative
    // filtering, with a tag-affinity cold-start); a signed-out viewer gets the
    // global-popular leaderboard. The recommender is a pure read of a
    // precomputed model, so a fresh account with too little history — or a model
    // not yet built — yields nothing; in that case we fall back to the popular
    // feed so the home is never blank. `score` overrides the row's vote tally
    // only for the leaderboard (recommendations keep the post's real net score).
    let mut items: Vec<(domain::Post, String, Option<i64>)> = Vec::new();
    if let Some(uid) = viewer_id {
        // Ask for enough to fill this page and reveal whether another follows.
        let want = page * FEED_PAGE_SIZE + 1;
        for rec in state
            .services
            .recommend_feed()
            .execute(uid, want)
            .await
            .unwrap_or_default()
        {
            items.push((rec.post, rec.community_slug, None));
        }
    }
    if items.is_empty() {
        for item in state.services.top_posts().await.unwrap_or_default() {
            items.push((item.post, item.community_slug, Some(item.score)));
        }
    }
    let (window, has_next) = paginate(items, page);
    let mut feed = Vec::new();
    for (post, slug, score) in window {
        let votable = can_vote(&state, viewer_id, post.demos_id).await;
        let mut row = post_row(&state, &post, viewer_id, votable, Some(slug)).await;
        if let Some(s) = score {
            row.score = s;
        }
        feed.push(row);
    }
    let pager = make_pager("/?", page, has_next, mode);

    // The lazy-loader asks for just the next slice of cards to append in place.
    if wants_fragment(&headers) {
        return render(FeedFragmentView {
            t: lang.strings(),
            posts: feed,
            pager,
        });
    }

    let mut demos = Vec::new();
    for d in state.services.demoi.list().await.unwrap_or_default() {
        let voters = state
            .services
            .memberships
            .voter_count(d.id)
            .await
            .unwrap_or(0);
        demos.push(DemosListItem {
            slug: d.slug,
            name: d.name,
            phase: phase(lang, Phase::from_voter_count(voters)).to_string(),
            voters,
        });
    }

    // Communities mid-founding — shown so anyone can help push one to quorum.
    let foundings = state
        .services
        .pending_foundings()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| FoundingListItem {
            id: p.id.0,
            slug: p.slug,
            name: p.name,
            signed: p.sign_offs.len(),
            required: domain::SIGN_OFFS_REQUIRED,
        })
        .collect();

    render(IndexView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: user.map(|u| u.handle),
        feed,
        pager,
        demos,
        foundings,
    })
}
