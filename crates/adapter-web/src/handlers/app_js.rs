//! Serve the main enhancement script.

use axum::{
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
};

pub async fn app_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/javascript; charset=utf-8"),
        )],
        include_str!("../../static/app.js"),
    )
        .into_response()
}
