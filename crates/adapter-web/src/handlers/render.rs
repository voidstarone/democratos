//! Render an Askama template to an HTTP response.

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

pub(crate) fn render<T: Template>(template: T) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            // Log the detail for operators, but never echo it to the client: a
            // rendering error can carry template/field internals, and leaking those
            // aids reconnaissance for no user benefit.
            eprintln!("template render error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal error rendering the page",
            )
                .into_response()
        }
    }
}
