//! The site-wide "top" feed handler.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
};

use crate::handlers::can_vote::can_vote;
use crate::handlers::current_user::current_user;
use crate::handlers::feed_query::FeedQuery;
use crate::handlers::make_pager::make_pager;
use crate::handlers::page_of::page_of;
use crate::handlers::paginate::paginate;
use crate::handlers::paging_mode::paging_mode;
use crate::handlers::post_row::post_row;
use crate::handlers::render::render;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::wants_fragment::wants_fragment;
use crate::views::feed_fragment_view::FeedFragmentView;
use crate::views::top_view::TopView;
use crate::AppState;

/// The site-wide "top" feed — most popular posts across all communities. Public
/// (works logged out); a signed-in viewer can vote on posts in communities they
/// belong to.
pub async fn top_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(pg): Query<FeedQuery>,
) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    let viewer_id = user.as_ref().map(|u| u.id);
    let page = page_of(pg.page);
    let mode = paging_mode(user.as_ref());

    let all = state.services.top_posts().await.unwrap_or_default();
    let (window, has_next) = paginate(all, page);
    let mut posts = Vec::new();
    for item in window {
        // The viewer may vote where they are a member in good standing.
        let votable = can_vote(&state, viewer_id, item.post.demos_id).await;
        let mut row = post_row(
            &state,
            &item.post,
            viewer_id,
            votable,
            Some(item.community_slug),
        )
        .await;
        row.score = item.score;
        posts.push(row);
    }
    let pager = make_pager("/top?", page, has_next, mode);

    if wants_fragment(&headers) {
        return render(FeedFragmentView {
            t: lang.strings(),
            posts,
            pager,
        });
    }

    render(TopView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: user.map(|u| u.handle),
        posts,
        pager,
    })
}
