//! Take a public invite request onto the waitlist.

use axum::{
    extract::{Form, State},
    http::HeaderMap,
    response::Response,
};

use app::RequestInviteError;

use crate::handlers::csrf_valid::csrf_valid;
use crate::handlers::issue_csrf::issue_csrf;
use crate::handlers::render_error::render_error;
use crate::handlers::render_with_cookie::render_with_cookie;
use crate::handlers::request_invite_form::RequestInviteForm;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::request_invite_view::RequestInviteView;
use crate::AppState;

/// Record a waitlist request. The service is idempotent and enumeration-safe, so
/// a success page is shown whether the email is new, already waiting, or already
/// an account — the form never reveals which. Only a malformed email is bounced
/// back inline.
pub async fn request_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<RequestInviteForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    if !csrf_valid(&headers, &form.csrf_token) {
        return render_error(lang, None, "session expired — please try again".to_string());
    }
    let (csrf_token, set_cookie) = issue_csrf(&headers, state.secure_cookies);

    let note = (!form.note.trim().is_empty()).then(|| form.note.trim());
    match state.invites.request_invite(&form.email, note).await {
        Ok(()) => render_with_cookie(
            RequestInviteView {
                t: lang.strings(),
                lang: lang.code(),
                current_user: None,
                csrf_token,
                submitted: true,
                error: None,
                email: String::new(),
                note: String::new(),
            },
            set_cookie,
        ),
        Err(RequestInviteError::Rejected(message)) => render_with_cookie(
            RequestInviteView {
                t: lang.strings(),
                lang: lang.code(),
                current_user: None,
                csrf_token,
                submitted: false,
                error: Some(message),
                email: form.email,
                note: form.note,
            },
            set_cookie,
        ),
        // A storage failure is not the visitor's fault; show a neutral error.
        Err(RequestInviteError::Store(_)) => render_error(
            lang,
            None,
            "could not record your request right now — please try again shortly".to_string(),
        ),
    }
}
