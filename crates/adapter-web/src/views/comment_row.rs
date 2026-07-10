pub struct CommentRow {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub depth: usize,
    pub removed: bool,
    /// Net upvote score (upvotes − downvotes).
    pub score: i64,
    pub voted_up: bool,
    pub voted_down: bool,
    /// Whether the viewer may vote on comments here (member in good standing).
    pub votable: bool,
}
