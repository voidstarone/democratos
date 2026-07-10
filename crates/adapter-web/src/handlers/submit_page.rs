//! The global "create a post" composer page handler.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
};

use crate::handlers::current_user::current_user;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::submit_query::SubmitQuery;
use crate::views::community_option::CommunityOption;
use crate::views::submit_view::SubmitView;
use crate::AppState;

/// The global "create a post" composer. Lists the communities the signed-in user
/// may post in as a picker; `?demos=slug` preselects one.
pub async fn submit_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SubmitQuery>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in to post".to_string());
    };
    let prefill = q.demos.unwrap_or_default();
    let memberships = state
        .services
        .memberships
        .list_for_user(user.id)
        .await
        .unwrap_or_default();
    let mut communities = Vec::new();
    for m in memberships {
        // Only offer communities whose posting policy currently admits this user
        // (member/voter/popularity — see `Services::can_post`).
        if !state
            .services
            .can_post(user.id, m.demos_id)
            .await
            .unwrap_or(false)
        {
            continue;
        }
        if let Ok(Some(d)) = state.services.demoi.get(m.demos_id).await {
            let selected = d.slug == prefill;
            communities.push(CommunityOption {
                slug: d.slug,
                name: d.name,
                selected,
            });
        }
    }
    if communities.is_empty() {
        return render_error(
            lang,
            Some(user.handle),
            "you can't post in any of your communities yet".to_string(),
        );
    }
    // If the prefill matched nothing (or was absent), default to the first.
    if !communities.iter().any(|c| c.selected) {
        communities[0].selected = true;
    }
    render(SubmitView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: Some(user.handle),
        communities,
    })
}
