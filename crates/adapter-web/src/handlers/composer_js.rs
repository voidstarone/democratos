//! Serve the composer's enhancement script.

use axum::{
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
};

/// The composer's enhancement script, served as a static file so the CSP can
/// forbid inline scripts (`script-src 'self'`). It was previously an inline
/// `<script>` in `submit.html`; its few translated strings now arrive via
/// `data-*` attributes on the form so the file stays static.
pub async fn composer_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/javascript; charset=utf-8"),
        )],
        include_str!("../../static/composer.js"),
    )
        .into_response()
}
