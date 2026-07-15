//! The account preferences page handler.

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::current_user::current_user;
use crate::handlers::paging_str::paging_str;
use crate::handlers::render::render;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::preferences_view::PreferencesView;
use crate::AppState;

/// The account preferences page (signed-in only). A signed-out visitor is sent to
/// sign in — there is no account to hold a preference.
pub async fn preferences_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return Redirect::to("/signin").into_response();
    };
    render(PreferencesView {
        t: lang.strings(),
        lang: lang.code(),
        feed_paging: paging_str(user.feed_paging),
        is_sensitive_reviewer: user.is_sensitive_reviewer,
        allows_mention_alerts: user.allows_mention_alerts,
        allows_jury_alerts: user.allows_jury_alerts,
        allows_trial_comment_alerts: user.allows_trial_comment_alerts,
        current_user: Some(user.handle),
        saved: false,
    })
}
