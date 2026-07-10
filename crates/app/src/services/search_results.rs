//! What a search turned up.

use domain::{Demos, Post};

/// What a search turned up: matching posts and (site-wide only) communities.
#[derive(Clone, Debug, Default)]
pub struct SearchResults {
    pub posts: Vec<Post>,
    pub communities: Vec<Demos>,
}
