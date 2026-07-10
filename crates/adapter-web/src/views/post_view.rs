use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::comment_row::CommentRow;
use crate::views::media_item::MediaItem;
use crate::views::rule_view::RuleView;

#[derive(Template)]
#[template(path = "post.html")]
pub struct PostView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    /// A voter may open a proposal to remove this post.
    pub viewer_is_voter: bool,
    pub demos_slug: String,
    pub id: u64,
    pub title: String,
    pub author: String,
    pub kind_label: String,
    /// Text body (empty for media-only posts).
    pub body: String,
    /// Every media attachment, rendered in order.
    pub media: Vec<MediaItem>,
    pub tags: Vec<String>,
    pub score: i64,
    pub voted_up: bool,
    pub voted_down: bool,
    pub votable: bool,
    pub removed: bool,
    pub is_nsfw: bool,
    pub viewer_can_post: bool,
    /// The community's active rules, offered as the report form's
    /// "which rule does it break?" dropdown.
    pub rules: Vec<RuleView>,
    /// Comment tree flattened to rows with an indentation depth.
    pub comments: Vec<CommentRow>,
}
