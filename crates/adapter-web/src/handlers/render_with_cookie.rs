//! Render a template and attach a freshly-minted `Set-Cookie`.

use askama::Template;
use axum::{
    http::{header, HeaderValue},
    response::Response,
};

use crate::handlers::render::render;

/// Render `view` and attach the `Set-Cookie` from
/// [`issue_csrf`](crate::handlers::issue_csrf) if one was minted, so the token in
/// the form and the token in the cookie always agree.
pub(crate) fn render_with_cookie<T: Template>(view: T, set_cookie: Option<String>) -> Response {
    let mut resp = render(view);
    if let Some(cookie) = set_cookie {
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            resp.headers_mut().append(header::SET_COOKIE, value);
        }
    }
    resp
}
