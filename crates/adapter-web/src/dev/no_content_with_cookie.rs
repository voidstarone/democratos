use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

/// Attach a `Set-Cookie` to an otherwise empty `204` so the switch takes effect
/// without navigating away; the dev bar reloads the page in place.
pub fn no_content_with_cookie(cookie: String) -> Response {
    let mut resp = StatusCode::NO_CONTENT.into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, value);
    }
    resp
}
