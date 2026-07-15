//! The set of accounts a viewer has blocked, for read-time content filtering.

use std::collections::HashSet;

use domain::UserId;

use crate::AppState;

/// The set of users `viewer` has personally blocked. A signed-out viewer (or any
/// lookup error) yields the empty set — blocking is best-effort presentation
/// filtering, never a hard gate. Feeds and threads retain only authors absent
/// from this set, so a blocked account's content never reaches the viewer.
pub(crate) async fn viewer_blocks(state: &AppState, viewer: Option<UserId>) -> HashSet<UserId> {
    match viewer {
        Some(uid) => state
            .services
            .blocked_by(uid)
            .await
            .map(|ids| ids.into_iter().collect())
            .unwrap_or_default(),
        None => HashSet::new(),
    }
}
