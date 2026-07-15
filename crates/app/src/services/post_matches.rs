use domain::Post;

/// Does a single lowercase query token match this post? True if it's a
/// substring of the title or body, or equals one of its tags.
pub(super) fn post_matches(post: &Post, token: &str) -> bool {
    post.title.to_lowercase().contains(token)
        || post.text_content().to_lowercase().contains(token)
        || post.tags.iter().any(|t| t == token)
}
