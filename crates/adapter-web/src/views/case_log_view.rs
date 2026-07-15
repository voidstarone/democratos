//! A community's public case log: every trial, open and past.

use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::case_row::CaseRow;

/// The public case log at `/d/:slug/trials` — every trial a community has held,
/// ongoing or concluded, newest first. Public record (works signed out), but
/// deliberately reached only via a quiet link, never the main navigation.
#[derive(Template)]
#[template(path = "case_log.html")]
pub struct CaseLogView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub slug: String,
    pub name: String,
    pub cases: Vec<CaseRow>,
}
