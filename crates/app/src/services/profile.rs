

use domain::{
    Comment, Post, UserId,
};


use crate::Result;


use super::services::Services;

impl Services {
    /// Every non-removed post by `author`, newest first. Filters the site-wide
    /// list (the same source search uses) — fine at a profile's scale.
    pub async fn posts_by_author(&self, author: UserId) -> Result<Vec<Post>> {
        let mut posts: Vec<Post> = self
            .posts
            .list_all()
            .await?
            .into_iter()
            .filter(|p| p.author == author && !p.removed)
            .collect();
        posts.sort_by(|a, b| b.created_at.0.cmp(&a.created_at.0));
        Ok(posts)
    }

    /// Every non-removed comment by `author`, newest first.
    pub async fn comments_by_author(&self, author: UserId) -> Result<Vec<Comment>> {
        let mut comments: Vec<Comment> = self
            .comments
            .list_by_author(author)
            .await?
            .into_iter()
            .filter(|c| !c.removed)
            .collect();
        comments.sort_by(|a, b| b.created_at.0.cmp(&a.created_at.0));
        Ok(comments)
    }
}
