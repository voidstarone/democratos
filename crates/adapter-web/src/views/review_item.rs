use crate::views::media_item::MediaItem;

/// One flagged item awaiting review, as the review console renders it.
pub struct ReviewItem {
    pub case_id: u64,
    /// `"post"` or `"comment"` — drives what the reveal shows.
    pub kind: &'static str,
    /// Present for a post target (empty for a comment).
    pub title: String,
    /// Body text of the post or comment.
    pub body: String,
    /// A post's media attachments (empty for a comment target).
    pub media: Vec<MediaItem>,
    /// How many reviewers have classified this case so far, and how many are
    /// needed to resolve it.
    pub votes: usize,
    pub quorum: usize,
    /// Whether the current reviewer has already cast a classification here.
    pub already_voted: bool,
    /// A short note the flagger left (may be empty).
    pub note: String,
}
