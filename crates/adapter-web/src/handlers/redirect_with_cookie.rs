//! Redirect while attaching a `Set-Cookie` header.

use axum::{
    http::{header, HeaderValue},
    response::{IntoResponse, Redirect, Response},
};

pub(crate) fn redirect_with_cookie(to: &str, cookie: String) -> Response {
    let mut resp = Redirect::to(to).into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, value);
    }
    resp
}
