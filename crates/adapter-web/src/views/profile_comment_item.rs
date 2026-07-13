//! One comment row on a profile page.

/// A single comment in a profile's Comments tab: its body and a permalink back to
/// the post it lives under.
pub struct ProfileCommentItem {
    pub post_id: u64,
    pub body: String,
}
