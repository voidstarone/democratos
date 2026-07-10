//! The `?page=` query the simple post feeds take.

use serde::Deserialize;

/// The one query parameter the simple post feeds (home, `/top`, a community)
/// take: which page to render. Absent on a first visit.
#[derive(Deserialize)]
pub struct FeedQuery {
    #[serde(default)]
    pub(crate) page: Option<u32>,
}
