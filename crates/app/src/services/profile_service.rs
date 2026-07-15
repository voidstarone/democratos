//! Member-profile use-cases: a member's own posts and comments, newest first.
//! Owns only the post and comment ports, so a profile view doesn't depend on the
//! whole app surface.

use std::sync::Arc;

use domain::{Comment, Post, UserId};

use crate::{CommentStore, PostStore, Result};

/// Member-profile listing use-cases, over just the post and comment stores.
#[derive(Clone)]
pub struct ProfileService {
    posts: Arc<dyn PostStore>,
    comments: Arc<dyn CommentStore>,
}

impl ProfileService {
    pub fn new(posts: Arc<dyn PostStore>, comments: Arc<dyn CommentStore>) -> Self {
        Self { posts, comments }
    }

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
