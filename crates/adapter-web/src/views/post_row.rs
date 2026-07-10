pub struct PostRow {
    pub id: u64,
    pub title: String,
    pub kind: String,
    pub author: String,
    pub tags: Vec<String>,
    /// First attachment's URL, shown as a small thumbnail in the feed (image or
    /// video). `None` for a text-only post.
    pub thumb: Option<String>,
    /// Whether [`thumb`](Self::thumb) is a video (render a `<video>` preview).
    pub thumb_is_video: bool,
    /// A short plain-text snippet of the post's body/caption for the card.
    pub snippet: String,
    /// Net upvote score (upvotes − downvotes).
    pub score: i64,
    /// The viewer's current vote (drives arrow highlight).
    pub voted_up: bool,
    pub voted_down: bool,
    /// Whether the viewer may vote here (member of this post's community).
    pub votable: bool,
    /// Community slug — set only in the cross-community home feed.
    pub community: Option<String>,
    pub removed: bool,
    pub is_nsfw: bool,
}
