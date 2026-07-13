//! The invite-accept page: validate the token, then show the finish-signup form.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
};

use crate::handlers::accept_query::AcceptQuery;
use crate::handlers::issue_csrf::issue_csrf;
use crate::handlers::render_error::render_error;
use crate::handlers::render_with_cookie::render_with_cookie;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::accept_invite_view::AcceptInviteView;
use crate::AppState;

/// Validate the `?token=` and, if it is still redeemable, render the finish-signup
/// form bound to the invited email. An unknown, expired, or already-used token
/// gets the opaque error page — no hint which it was.
pub async fn accept_invite_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AcceptQuery>,
) -> Response {
    let lang = resolve_lang(&headers);
    let request = match state.services.validate_invite_token(&query.token).await {
        Ok(request) => request,
        Err(_) => {
            return render_error(
                lang,
                None,
                "this invite link is invalid or has expired".to_string(),
            )
        }
    };
    let (csrf_token, set_cookie) = issue_csrf(&headers, state.secure_cookies);
    render_with_cookie(
        AcceptInviteView {
            t: lang.strings(),
            lang: lang.code(),
            current_user: None,
            email: request.email,
            token: query.token,
            csrf_token,
            error: None,
            handle: String::new(),
        },
        set_cookie,
    )
}
