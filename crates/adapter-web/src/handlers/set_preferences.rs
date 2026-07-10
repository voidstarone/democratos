//! Save the feed-delivery preference.

use axum::{
    extract::{Form, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::FeedPaging;

use crate::handlers::current_user::current_user;
use crate::handlers::paging_str::paging_str;
use crate::handlers::preferences_form::PreferencesForm;
use crate::handlers::render::render;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::preferences_view::PreferencesView;
use crate::AppState;

/// Save the feed-delivery preference for the signed-in account, then re-render the
/// page with a confirmation. A plain form POST, so it works with no JavaScript.
pub async fn set_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<PreferencesForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return Redirect::to("/signin").into_response();
    };
    let paging = match form.feed_paging.as_str() {
        "pages" => FeedPaging::Pages,
        "lazy" => FeedPaging::Lazy,
        _ => FeedPaging::Auto,
    };
    let saved = state.services.set_feed_paging(user.id, paging).await.is_ok();
    render(PreferencesView {
        t: lang.strings(),
        lang: lang.code(),
        // Reflect what we just stored (the in-hand `user` still holds the old value).
        feed_paging: paging_str(if saved { paging } else { user.feed_paging }),
        current_user: Some(user.handle),
        saved,
    })
}
