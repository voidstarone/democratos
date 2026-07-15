//! Facade delegators for member-profile use-cases. The logic now lives in
//! [`ProfileService`](super::profile_service::ProfileService); these thin methods
//! keep `services.posts_by_author()` and friends working for call sites not yet
//! migrated off the `Services` aggregator.

use domain::{Comment, Post, UserId};

use crate::Result;

use super::profile_service::ProfileService;
use super::services::Services;

impl Services {
    /// Build the extracted [`ProfileService`] from the ports this aggregator
    /// still holds. Cheap — `Arc` clones only — so delegators construct one per
    /// call rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `ProfileService` directly.
    pub(super) fn profile_service(&self) -> ProfileService {
        ProfileService::new(self.posts.clone(), self.comments.clone())
    }

    /// Every non-removed post by `author`, newest first. Filters the site-wide
    /// list (the same source search uses) — fine at a profile's scale.
    pub async fn posts_by_author(&self, author: UserId) -> Result<Vec<Post>> {
        self.profile_service().posts_by_author(author).await
    }

    /// Every non-removed comment by `author`, newest first.
    pub async fn comments_by_author(&self, author: UserId) -> Result<Vec<Comment>> {
        self.profile_service().comments_by_author(author).await
    }
}
