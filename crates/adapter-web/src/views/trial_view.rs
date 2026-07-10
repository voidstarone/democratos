use askama::Template;

use crate::i18n::strings::Strings;

#[derive(Template)]
#[template(path = "trial.html")]
pub struct TrialView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub id: u64,
    pub accused: String,
    pub jurors: Vec<String>,
    pub verdict: String,
    pub open: bool,
    pub guilty: u64,
    pub not_guilty: u64,
    pub viewer_is_juror: bool,
}
