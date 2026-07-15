//! Search use-cases: full-text-ish search over posts and, site-wide,
//! communities. Owns only the post and community ports, so a search box doesn't
//! depend on the whole app surface.

use std::sync::Arc;

use domain::normalize_tags;

use crate::{DemosStore, PostStore, Result};

use super::post_matches::post_matches;
use super::search_results::SearchResults;
use super::search_scope::SearchScope;

/// Search use-cases, over just the post and community stores.
#[derive(Clone)]
pub struct SearchService {
    posts: Arc<dyn PostStore>,
    demoi: Arc<dyn DemosStore>,
}

impl SearchService {
    pub fn new(posts: Arc<dyn PostStore>, demoi: Arc<dyn DemosStore>) -> Self {
        Self { posts, demoi }
    }

    /// Full-text-ish search over posts (title / body / tags) and, site-wide,
    /// communities (name / slug). A post matches if **any** query token is a
    /// substring of its title or body; an optional `tag` filter additionally
    /// requires that exact tag. When a `tag` is given the candidate set comes
    /// from the store's pipe-wrapped tag index ([`PostStore::by_tag`] /
    /// [`DemosStore::by_tag`]) — an indexed lookup rather than a scan of every
    /// row — and any query tokens then narrow those matches. Removed and
    /// pending-review posts are excluded.
    pub async fn search(
        &self,
        query: &str,
        scope: SearchScope,
        tag: Option<&str>,
    ) -> Result<SearchResults> {
        let tokens: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
        // Normalize the tag filter the same way stored tags are, so it matches the
        // index exactly and can never carry a `LIKE` metacharacter into the store.
        let tag = tag.and_then(|t| normalize_tags(t).into_iter().next());

        // With a tag filter, fetch the tagged rows straight from the index; without
        // one, fall back to the full candidate list the tokens then filter.
        let candidates = match (&tag, scope) {
            (Some(t), SearchScope::All) => self.posts.by_tag(None, t).await?,
            (Some(t), SearchScope::Demos(id)) => self.posts.by_tag(Some(id), t).await?,
            (None, SearchScope::All) => self.posts.list_all().await?,
            (None, SearchScope::Demos(id)) => self.posts.list(id).await?,
        };
        let posts = candidates
            .into_iter()
            .filter(|p| !p.removed && !p.pending_review)
            .filter(|p| tokens.is_empty() || tokens.iter().any(|tok| post_matches(p, tok)))
            .collect();

        // Communities are only searched in the site-wide scope. A tag filter looks
        // them up by the same index; otherwise they match on name/slug tokens.
        let communities = if !matches!(scope, SearchScope::All) {
            Vec::new()
        } else if let Some(t) = &tag {
            self.demoi
                .by_tag(t)
                .await?
                .into_iter()
                .filter(|d| {
                    tokens.is_empty() || {
                        let name = d.name.to_lowercase();
                        let slug = d.slug.to_lowercase();
                        tokens.iter().any(|tok| name.contains(tok) || slug.contains(tok))
                    }
                })
                .collect()
        } else if !tokens.is_empty() {
            self.demoi
                .list()
                .await?
                .into_iter()
                .filter(|d| {
                    let name = d.name.to_lowercase();
                    let slug = d.slug.to_lowercase();
                    tokens.iter().any(|t| name.contains(t) || slug.contains(t))
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(SearchResults { posts, communities })
    }
}
