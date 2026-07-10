//! Whether a viewer may up/down vote on a post in a given community.

use domain::{DemosId, UserId};

use crate::AppState;

/// Whether `viewer` may up/down vote on a post in `demos`: only a signed-in
/// member in good standing (not sanctioned). A signed-out viewer never can. Used
/// by the cross-community feeds (home + `/top`) where each row may belong to a
/// community the viewer isn't a member of.
pub(crate) async fn can_vote(state: &AppState, viewer: Option<UserId>, demos: DemosId) -> bool {
    match viewer {
        Some(uid) => state
            .services
            .memberships
            .get(uid, demos)
            .await
            .ok()
            .flatten()
            .map(|m| !m.sanctioned)
            .unwrap_or(false),
        None => false,
    }
}
