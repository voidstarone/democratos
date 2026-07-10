use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::report_row::ReportRow;

#[derive(Template)]
#[template(path = "reports.html")]
pub struct ReportsView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub slug: String,
    pub reports: Vec<ReportRow>,
}
