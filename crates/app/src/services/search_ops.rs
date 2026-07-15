//! Facade delegator for search use-cases. The logic now lives in
//! [`SearchService`](super::search_service::SearchService); this thin method
//! keeps `services.search()` working for call sites not yet migrated off the
//! `Services` aggregator.

use crate::Result;

use super::search_results::SearchResults;
use super::search_scope::SearchScope;
use super::search_service::SearchService;
use super::services::Services;

impl Services {
    /// Build the extracted [`SearchService`] from the ports this aggregator still
    /// holds. Cheap — `Arc` clones only — so the delegator constructs one per call
    /// rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `SearchService` directly.
    pub(super) fn search_service(&self) -> SearchService {
        SearchService::new(self.posts.clone(), self.demoi.clone())
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
        self.search_service().search(query, scope, tag).await
    }
}
