use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::review_item::ReviewItem;

/// The sensitive-content review console — the platform-wide (extra-demos) queue,
/// visible only to opted-in reviewers.
#[derive(Template)]
#[template(path = "review.html")]
pub struct ReviewView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub items: Vec<ReviewItem>,
}
