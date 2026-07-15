pub struct CommentRow {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub depth: usize,
    pub removed: bool,
    /// Whether the viewer has blocked this comment's author. The row is kept (so
    /// replies from others stay threaded) but its body is replaced with a muted
    /// placeholder and voting is suppressed.
    pub is_blocked: bool,
    /// Net upvote score (upvotes − downvotes).
    pub score: i64,
    pub voted_up: bool,
    pub voted_down: bool,
    /// Whether the viewer may vote on comments here (member in good standing).
    pub votable: bool,
}
