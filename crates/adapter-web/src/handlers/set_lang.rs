//! Set the UI-language cookie.

use axum::{
    extract::{Form, State},
    http::HeaderMap,
    response::Response,
};

use crate::handlers::lang_form::LangForm;
use crate::handlers::redirect_with_cookie::redirect_with_cookie;
use crate::handlers::safe_referer_back::safe_referer_back;
use crate::handlers::secure_attr::secure_attr;
use crate::i18n::lang::Lang;
use crate::AppState;

pub async fn set_lang(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<LangForm>,
) -> Response {
    let back = safe_referer_back(&headers);
    let code = if Lang::from_code(&form.lang).is_some() {
        form.lang
    } else {
        "en".to_string()
    };
    redirect_with_cookie(
        &back,
        format!(
            "lang={code}; Path=/; SameSite=Lax{}",
            secure_attr(state.secure_cookies)
        ),
    )
}
