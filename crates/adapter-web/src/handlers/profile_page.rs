//! A user's public profile page: their posts or comments, tab-selected.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Response,
};

use crate::handlers::current_user::current_user;
use crate::handlers::profile_query::ProfileQuery;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::profile_comment_item::ProfileCommentItem;
use crate::views::profile_post_item::ProfilePostItem;
use crate::views::profile_view::ProfileView;
use crate::AppState;

/// `GET /u/:handle` — a user's profile. Shows their Posts by default, or their
/// Comments with `?tab=comments`. Only the selected tab's data is fetched; the
/// tabs are ordinary links, so it works with JavaScript disabled.
pub async fn profile_page(
    State(state): State<AppState>,
    Path(handle): Path<String>,
    Query(query): Query<ProfileQuery>,
    headers: HeaderMap,
) -> Response {
    let lang = resolve_lang(&headers);
    let viewer_user = current_user(&state, &headers).await;
    let viewer = viewer_user.as_ref().map(|u| u.handle.clone());

    let user = match state.accounts.user_by_handle(&handle).await {
        Ok(Some(u)) => u,
        Ok(None) => return render_error(lang, viewer, "no such user".to_string()),
        Err(e) => return render_error(lang, viewer, e.to_string()),
    };

    // A signed-in viewer can block anyone but themselves; the button flips to
    // "Unblock" once the block is in force.
    let is_self = viewer_user.as_ref().map(|u| u.id) == Some(user.id);
    let is_blocked = match &viewer_user {
        Some(v) if !is_self => v.blocks(user.id),
        _ => false,
    };
    let can_block = viewer_user.is_some() && !is_self;

    let show_comments = query.tab.as_deref() == Some("comments");
    let mut posts = Vec::new();
    let mut comments = Vec::new();

    if show_comments {
        match state.profile.comments_by_author(user.id).await {
            Ok(cs) => {
                comments = cs
                    .into_iter()
                    .map(|c| ProfileCommentItem {
                        post_id: c.post_id.0,
                        body: c.body,
                    })
                    .collect()
            }
            Err(e) => return render_error(lang, viewer, e.to_string()),
        }
    } else {
        match state.profile.posts_by_author(user.id).await {
            Ok(ps) => {
                posts = ps
                    .into_iter()
                    .map(|p| ProfilePostItem {
                        id: p.id.0,
                        title: p.title,
                    })
                    .collect()
            }
            Err(e) => return render_error(lang, viewer, e.to_string()),
        }
    }

    render(ProfileView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: viewer,
        handle: user.handle,
        tab: if show_comments { "comments" } else { "posts" }.to_string(),
        can_block,
        is_blocked,
        posts,
        comments,
    })
}
