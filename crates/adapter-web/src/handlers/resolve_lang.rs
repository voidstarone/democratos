//! Resolve the active UI language from request headers.

use axum::http::{header, HeaderMap};

use crate::handlers::cookie_value::cookie_value;
use crate::i18n::lang::Lang;

/// Locale resolution: an explicit `lang` cookie wins, else `Accept-Language`,
/// else English.
pub(crate) fn resolve_lang(headers: &HeaderMap) -> Lang {
    if let Some(code) = cookie_value(headers, "lang") {
        if let Some(lang) = Lang::from_code(&code) {
            return lang;
        }
    }
    headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .map(Lang::from_accept_language)
        .unwrap_or(Lang::En)
}
