/// Errors talking to a peer's feed.
#[derive(Debug)]
pub enum FeedError {
    Transport(String),
    Status(u16),
}

impl std::fmt::Display for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedError::Transport(e) => write!(f, "feed transport: {e}"),
            FeedError::Status(c) => write!(f, "feed returned HTTP {c}"),
        }
    }
}
