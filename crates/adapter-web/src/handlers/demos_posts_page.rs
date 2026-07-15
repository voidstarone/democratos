//! A community's posts, newest first, sliced to the requested page.

use std::collections::HashSet;

use domain::{DemosId, UserId};

use crate::handlers::paginate::paginate;
use crate::AppState;

/// A community's posts, newest first, sliced to the requested page. Newest-first
/// gives pagination a stable order independent of store iteration order. Posts by
/// an author in `blocked` are dropped before paging, so the viewer never sees a
/// blocked account and pages stay full.
pub(crate) async fn demos_posts_page(
    state: &AppState,
    demos: DemosId,
    page: usize,
    blocked: &HashSet<UserId>,
) -> app::Result<(Vec<domain::Post>, bool)> {
    let mut all = state.content.list_posts(demos).await?;
    all.retain(|p| !blocked.contains(&p.author));
    all.sort_by(|a, b| b.created_at.0.cmp(&a.created_at.0));
    Ok(paginate(all, page))
}
