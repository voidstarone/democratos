use axum::{
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
};

/// Serve the dev bar script. Always served (it self-gates by calling
/// [`accounts`](crate::dev::accounts::accounts) and doing nothing when that 404s), so production never special-
/// cases it.
pub async fn dev_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/javascript; charset=utf-8"),
        )],
        include_str!("../../static/dev.js"),
    )
        .into_response()
}
