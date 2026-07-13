//! A post: title, optional body, and ordered media attachments.

use serde::{Deserialize, Serialize};

use crate::content::media::Media;
use crate::{DemosId, PostId, UserId};
use crate::time::Timestamp;

/// A post: a title, an optional free-text body, and any number of ordered media
/// attachments. Body and media are independent — a post may have either, both,
/// or (media-only) just attachments.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(from = "PostWire")]
pub struct Post {
    pub id: PostId,
    pub demos_id: DemosId,
    pub author: UserId,
    pub title: String,
    /// Free-text body; empty for a media-only post.
    #[serde(default)]
    pub body: String,
    /// Ordered media attachments; empty for a text-only post.
    #[serde(default)]
    pub media: Vec<Media>,
    /// Normalized, deduped tags (see [`crate::content::normalize_tags`]).
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: Timestamp,
    /// Set when removed by a moderation decision or upheld report.
    #[serde(default)]
    pub removed: bool,
    /// Flagged as not-safe-for-work by the detector (or an explicit `nsfw` tag).
    /// Flagging blurs/age-gates the post; it never removes it — see [`crate::nsfw`].
    #[serde(default)]
    pub is_nsfw: bool,
    /// Flagged sensitive by a user and hidden from normal feeds while a
    /// platform-wide review case gathers reviewer classifications
    /// ([`crate::sensitive`]). Distinct from [`removed`](Self::removed): a review
    /// that clears the flag sets this back to `false`; one that upholds it sets
    /// `removed`.
    #[serde(default)]
    pub pending_review: bool,
}

impl Post {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: PostId,
        demos_id: DemosId,
        author: UserId,
        title: impl Into<String>,
        body: impl Into<String>,
        media: Vec<Media>,
        tags: Vec<String>,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            demos_id,
            author,
            title: title.into(),
            body: body.into(),
            media,
            tags,
            created_at,
            removed: false,
            is_nsfw: false,
            pending_review: false,
        }
    }

    /// The attachment to show as a thumbnail/hero — the first one, if any.
    pub fn primary_media(&self) -> Option<&Media> {
        self.media.first()
    }

    /// A coarse display label: `text` (no media), `image`/`video` (one item),
    /// or `gallery` (several).
    pub fn kind_label(&self) -> &'static str {
        match self.media.as_slice() {
            [] => "text",
            [one] => one.kind_label(),
            _ => "gallery",
        }
    }

    /// All the free text a search or NSFW scan should consider: the body plus
    /// every media caption.
    pub fn text_content(&self) -> String {
        let mut parts = Vec::with_capacity(1 + self.media.len());
        if !self.body.is_empty() {
            parts.push(self.body.clone());
        }
        for m in &self.media {
            if !m.caption.is_empty() {
                parts.push(m.caption.clone());
            }
        }
        parts.join(" ")
    }
}

/// Deserialization shim that also accepts the pre-`media` on-disk shape (a single
/// externally-tagged `kind`), migrating it into `body`/`media` so older datasets
/// keep loading. New data has `body`/`media` and no `kind`.
#[derive(Deserialize)]
struct PostWire {
    id: PostId,
    demos_id: DemosId,
    author: UserId,
    title: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    media: Vec<Media>,
    #[serde(default)]
    tags: Vec<String>,
    created_at: Timestamp,
    #[serde(default)]
    removed: bool,
    #[serde(default)]
    is_nsfw: bool,
    #[serde(default)]
    pending_review: bool,
    /// Legacy `PostKind` field, present only in pre-`media` datasets.
    #[serde(default)]
    kind: Option<LegacyKind>,
}

/// The old `PostKind`, kept solely to migrate legacy rows in [`PostWire`].
#[derive(Deserialize)]
enum LegacyKind {
    Text { body: String },
    Image { url: String, caption: String },
    Video { url: String, caption: String },
}

impl From<PostWire> for Post {
    fn from(w: PostWire) -> Self {
        let mut body = w.body;
        let mut media = w.media;
        match w.kind {
            Some(LegacyKind::Text { body: b }) if body.is_empty() => body = b,
            Some(LegacyKind::Image { url, caption }) => media.push(Media::image(url, caption)),
            Some(LegacyKind::Video { url, caption }) => media.push(Media::video(url, caption)),
            _ => {}
        }
        Post {
            id: w.id,
            demos_id: w.demos_id,
            author: w.author,
            title: w.title,
            body,
            media,
            tags: w.tags,
            created_at: w.created_at,
            removed: w.removed,
            is_nsfw: w.is_nsfw,
            pending_review: w.pending_review,
        }
    }
}
