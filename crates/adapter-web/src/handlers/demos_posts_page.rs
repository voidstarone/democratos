//! A community's posts, newest first, sliced to the requested page.

use domain::DemosId;

use crate::handlers::paginate::paginate;
use crate::AppState;

/// A community's posts, newest first, sliced to the requested page. Newest-first
/// gives pagination a stable order independent of store iteration order.
pub(crate) async fn demos_posts_page(
    state: &AppState,
    demos: DemosId,
    page: usize,
) -> app::Result<(Vec<domain::Post>, bool)> {
    let mut all = state.services.list_posts(demos).await?;
    all.sort_by(|a, b| b.created_at.0.cmp(&a.created_at.0));
    Ok(paginate(all, page))
}
