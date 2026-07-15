//! Open a founding petition.

use axum::{
    extract::{Form, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::current_user::current_user;
use crate::handlers::found_form::FoundForm;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// Open a founding petition from the display name. The demos is *not* created
/// here: a community is only born once nine other people sign off (see
/// [`sign_founding`](crate::handlers::sign_founding)). Redirects to the new
/// petition's page, whose shareable link gathers those sign-offs.
pub async fn start_founding(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<FoundForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    let tags = domain::normalize_tags(&form.tags);
    match state
        .services
        .start_founding_tagged(user.id, &form.name, tags)
        .await
    {
        Ok(p) => Redirect::to(&format!("/found/{}", p.id.0)).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
