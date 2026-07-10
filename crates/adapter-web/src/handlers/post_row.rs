//! Build a [`PostRow`] for a feed/search list from a domain post.

use domain::UserId;

use crate::handlers::handle_of::handle_of;
use crate::views::post_row::PostRow;
use crate::AppState;

/// How many characters of body/caption a feed card shows before an ellipsis.
const SNIPPET_LEN: usize = 140;

/// A short one-line preview of a post for a feed card: its body, or — for a
/// media-only post — its first caption. Truncated on a char boundary.
fn snippet_of(p: &domain::Post) -> String {
    let source = if !p.body.is_empty() {
        p.body.as_str()
    } else {
        p.media
            .iter()
            .find(|m| !m.caption.is_empty())
            .map(|m| m.caption.as_str())
            .unwrap_or("")
    };
    let source = source.split_whitespace().collect::<Vec<_>>().join(" ");
    match source.char_indices().nth(SNIPPET_LEN) {
        Some((idx, _)) => format!("{}…", &source[..idx].trim_end()),
        None => source,
    }
}

/// Build a [`PostRow`] for a feed/search list from a domain post. `votable`
/// says whether the viewer may up/down vote here; `community` is set only for
/// the cross-community home feed.
pub(crate) async fn post_row(
    state: &AppState,
    p: &domain::Post,
    viewer: Option<UserId>,
    votable: bool,
    community: Option<String>,
) -> PostRow {
    let thumb = p.primary_media().map(|m| m.url.clone());
    let thumb_is_video = p.primary_media().map(|m| m.is_video).unwrap_or(false);
    let snippet = snippet_of(p);
    let score = state.services.post_score(p.id).await.unwrap_or(0);
    let viewer_vote = match viewer {
        Some(u) => state.services.user_post_vote(p.id, u).await.unwrap_or(None),
        None => None,
    };
    PostRow {
        id: p.id.0,
        title: p.title.clone(),
        kind: p.kind_label().to_string(),
        author: handle_of(state, p.author).await,
        tags: p.tags.clone(),
        thumb,
        thumb_is_video,
        snippet,
        score,
        voted_up: viewer_vote == Some(true),
        voted_down: viewer_vote == Some(false),
        votable,
        community,
        removed: p.removed,
        is_nsfw: p.is_nsfw,
    }
}
