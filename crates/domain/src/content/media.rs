//! A single media attachment on a post.

use serde::{Deserialize, Serialize};

/// A single media attachment on a post. The domain stores a URL reference —
/// never bytes; the bytes live behind the media store.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Media {
    pub url: String,
    #[serde(default)]
    pub caption: String,
    /// `true` for a video, `false` for an image — decides `<video>` vs `<img>`
    /// at render time and the `"video"`/`"image"` label a scanner receives.
    #[serde(default)]
    pub is_video: bool,
}

impl Media {
    pub fn image(url: impl Into<String>, caption: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            caption: caption.into(),
            is_video: false,
        }
    }

    pub fn video(url: impl Into<String>, caption: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            caption: caption.into(),
            is_video: true,
        }
    }

    /// `"video"` or `"image"` — the kind label a scanner/classifier expects.
    pub fn kind_label(&self) -> &'static str {
        if self.is_video {
            "video"
        } else {
            "image"
        }
    }
}
