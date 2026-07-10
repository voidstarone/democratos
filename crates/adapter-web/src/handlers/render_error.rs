//! Render the translated error page.

use axum::response::Response;

use crate::handlers::render::render;
use crate::i18n::lang::Lang;
use crate::views::error_view::ErrorView;

pub(crate) fn render_error(lang: Lang, current_user: Option<String>, message: String) -> Response {
    render(ErrorView {
        t: lang.strings(),
        lang: lang.code(),
        current_user,
        message,
    })
}
