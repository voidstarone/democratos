//! Clear the session cookie.

use axum::response::Response;

use crate::handlers::redirect_with_cookie::redirect_with_cookie;

pub async fn logout() -> Response {
    redirect_with_cookie("/", "uid=; Path=/; Max-Age=0".to_string())
}
